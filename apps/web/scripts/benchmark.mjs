#!/usr/bin/env node
// Run the shared Wasm loader on Chromium's throttled page target. CDP does not
// throttle dedicated workers, so timing the site's compute worker would not
// measure the reference profile (platform/verification).
import { spawn } from 'node:child_process';
import { copyFileSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { compare, percentile, table } from '../../../tools/perf/bench.mjs';

const web = fileURLToPath(new URL('..', import.meta.url));
const root = join(web, '../..');
const WARMUP = 50;
const SAMPLES = 1000;
const COLD_INIT_BUDGET_MS = 150; // platform/compute-core

function option(name) {
  const at = process.argv.indexOf(name);
  if (at < 0) return null;
  if (!process.argv[at + 1] || process.argv[at + 1].startsWith('--')) throw new Error(`${name} needs a path`);
  return process.argv[at + 1];
}

function serve() {
  const server = spawn(process.execPath, ['scripts/serve.mjs', '0'], { cwd: web });
  const origin = new Promise((resolveOrigin, reject) => {
    server.once('error', reject);
    server.once('exit', (code) => reject(new Error(`site server exited: ${code}`)));
    server.stdout.on('data', (chunk) => {
      const port = /localhost:(\d+)/.exec(chunk.toString())?.[1];
      if (port) resolveOrigin(`http://127.0.0.1:${port}`);
    });
  });
  return { server, origin };
}

async function main() {
  const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
  const profile = JSON.parse(readFileSync(join(root, 'data/reference-profile.json'), 'utf8'));
  const benchPath = `_benchmark-${process.pid}`;
  const benchDir = join(web, 'dist', benchPath);
  mkdirSync(benchDir);
  for (const name of ['module.mjs', 'harden.mjs', 'assets.mjs']) {
    copyFileSync(join(root, 'packages/runtime/src', name), join(benchDir, name));
  }
  const { server, origin } = serve();
  let browser;
  try {
    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
    page.setDefaultTimeout(120_000);
    await page.goto(`${await origin}/offline/`);
    const cdp = await page.context().newCDPSession(page);
    await cdp.send('Emulation.setCPUThrottlingRate', { rate: profile.cpu.slowdown });
    await page.evaluate(async (path) => {
      const { loadModule } = await import(`/${path}/module.mjs`);
      const { assetProvider } = await import(`/${path}/assets.mjs`);
      const registry = await fetch('/assets/registry.json').then((r) => r.json());
      const assets = assetProvider(registry, async (id, version, key) => {
        const response = await fetch(`/assets/${id}/${version}/${key}`);
        return response.ok ? response.arrayBuffer() : null;
      });
      const modules = new Map();
      window.__benchmark = {
        async get(name) {
          if (!modules.has(name)) {
            const bytes = await fetch(`/wasm/${name}.wasm`).then((r) => r.arrayBuffer());
            const start = performance.now();
            const module = await loadModule(bytes, name, { maxBytes: 50_000_000, assets });
            modules.set(name, { module, initMs: performance.now() - start });
          }
          return modules.get(name);
        },
      };
    }, benchPath);
    const tools = [];
    const modules = new Map();
    for (const [index, tool] of catalog.tools.entries()) {
      const example = tool.examples.find((e) => e.id === tool['x-primary-example']) ?? tool.examples[0];
      const stats = await page.evaluate(async ({ id, moduleName, input, warmup, samples }) => {
        const { module, initMs } = await window.__benchmark.get(moduleName);
        const invoke = () => module.invoke(id, input);
        for (let i = 0; i < warmup; i++) await invoke();
        const times = [];
        for (let i = 0; i < samples; i++) {
          const start = performance.now();
          const result = await invoke();
          const elapsed = performance.now() - start;
          if (!JSON.parse(result).ok) throw new Error(`${id} returned an error during the benchmark`);
          times.push(elapsed);
        }
        return { times, initMs };
      }, { id: tool.id, moduleName: tool.module, input: JSON.stringify(example.input), warmup: WARMUP, samples: SAMPLES });
      stats.times.sort((a, b) => a - b);
      tools.push({ id: tool.id, p50Ms: percentile(stats.times, 0.5), p95Ms: percentile(stats.times, 0.95) });
      modules.set(tool.module, stats.initMs);
      if ((index + 1) % 25 === 0) console.error(`measured ${index + 1}/${catalog.tools.length} tools`);
    }
    const report = {
      profile: profile.version, host: 'chromium-main-thread', cpuSlowdown: profile.cpu.slowdown,
      warmup: WARMUP, samples: SAMPLES,
      modules: [...modules].map(([name, initMs]) => ({ name, initMs })), tools,
    };
    const baseline = option('--baseline');
    const rows = baseline ? compare(report, JSON.parse(readFileSync(baseline, 'utf8'))) : tools;
    console.log(`Chromium benchmark: profile ${profile.version}, ${profile.cpu.slowdown}× CPU slowdown, ${WARMUP} warm-up and ${SAMPLES} measured calls per tool`);
    console.log(table(rows));
    const output = option('--output');
    if (output) writeFileSync(output, JSON.stringify(report, null, 2) + '\n');
    const slowModules = report.modules.filter((m) => m.initMs > COLD_INIT_BUDGET_MS);
    if (slowModules.length > 0) {
      console.error(`${slowModules.length} module(s) exceed the ${COLD_INIT_BUDGET_MS} ms cold-instantiation budget: ${slowModules.map((m) => m.name).join(', ')}`);
      process.exitCode = 1;
    }
    const regressions = rows.filter((r) => r.regression);
    if (regressions.length > 0) {
      console.error(`${regressions.length} tool(s) exceed the 20% p95 regression limit; see the table above`);
      process.exitCode = 1;
    }
  } finally {
    await browser?.close();
    server.kill();
    rmSync(benchDir, { recursive: true, force: true });
  }
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  main().catch((error) => { console.error(error); process.exitCode = 1; });
}
