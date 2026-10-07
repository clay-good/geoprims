#!/usr/bin/env node
// Run the shared Wasm loader on Chromium's throttled page target. CDP does not
// throttle dedicated workers, so timing the site's compute worker would not
// measure the reference profile (platform/verification).
import { spawn } from 'node:child_process';
import { copyFileSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { cpuRate } from './cpu.mjs';
import { compare, percentile, table } from '../../../tools/perf/bench.mjs';

const web = fileURLToPath(new URL('..', import.meta.url));
const root = join(web, '../..');
const WARMUP = 50;
const SAMPLES = 1000;
const COLD_INIT_BUDGET_MS = 150; // platform/compute-core
const CHAIN_WARMUP = 5;
const CHAIN_SAMPLES = 50;
// add-job-workflows "Performance": the whole mapping-flight chain, every step,
// within 300 ms on the reference profile.
const CHAIN_BUDGETS_MS = { 'mapping-flight': 300 };

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
  const { workflows } = JSON.parse(readFileSync(join(root, 'data/workflows.json'), 'utf8'));
  for (const name of ['module.mjs', 'harden.mjs', 'assets.mjs', 'chain.mjs']) {
    copyFileSync(join(root, 'packages/runtime/src', name), join(benchDir, name));
  }
  writeFileSync(join(benchDir, 'index.html'), '<!doctype html><html lang="en"><title>geoprims benchmark</title></html>');
  const { server, origin } = serve();
  let browser;
  try {
    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
    page.setDefaultTimeout(120_000);
    await page.goto(`${await origin}/${benchPath}/`);
    const cdp = await page.context().newCDPSession(page);
    const cpu = await cpuRate(browser, profile);
    await cdp.send('Emulation.setCPUThrottlingRate', { rate: cpu.rate });
    await page.evaluate(async (path) => {
      const { loadModule } = await import(`/${path}/module.mjs`);
      const { assetProvider } = await import(`/${path}/assets.mjs`);
      const registry = await fetch('/assets/registry.json').then((r) => r.json());
      const assets = assetProvider(registry, async (id, version, key) => {
        const response = await fetch(`/assets/${id}/${version}/${key}`);
        return response.ok ? response.arrayBuffer() : null;
      });
      const modules = new Map();
      const durations = new Map();
      window.__benchmark = {
        durations,
        async get(name) {
          if (!modules.has(name)) {
            const bytes = await fetch(`/wasm/${name}.wasm`).then((r) => r.arrayBuffer());
            const start = performance.now();
            const module = await loadModule(bytes, name, {
              maxBytes: 50_000_000, assets,
              onInvoke: (id, ms) => {
                if (!durations.has(id)) durations.set(id, []);
                durations.get(id).push(ms);
              },
            });
            modules.set(name, { module, initMs: performance.now() - start });
          }
          return modules.get(name);
        },
      };
    }, benchPath);
    const tools = [];
    const modules = new Map();
    const onlyWorkflows = process.argv.includes('--workflows-only');
    for (const [index, tool] of (onlyWorkflows ? [] : catalog.tools).entries()) {
      const example = tool.examples.find((e) => e.id === tool['x-primary-example']) ?? tool.examples[0];
      const stats = await page.evaluate(async ({ id, moduleName, input, warmup, samples }) => {
        const { module, initMs } = await window.__benchmark.get(moduleName);
        const invoke = () => module.invoke(id, input);
        for (let i = 0; i < warmup; i++) await invoke();
        window.__benchmark.durations.set(id, []);
        for (let i = 0; i < samples; i++) {
          const result = await invoke();
          if (!JSON.parse(result).ok) throw new Error(`${id} returned an error during the benchmark`);
        }
        const times = window.__benchmark.durations.get(id);
        if (times.length !== samples) throw new Error(`${id} produced ${times.length} timings for ${samples} calls`);
        window.__benchmark.durations.delete(id);
        return { times, initMs };
      }, { id: tool.id, moduleName: tool.module, input: JSON.stringify(example.input), warmup: WARMUP, samples: SAMPLES });
      stats.times.sort((a, b) => a - b);
      tools.push({ id: tool.id, p50Ms: percentile(stats.times, 0.5), p95Ms: percentile(stats.times, 0.95) });
      modules.set(tool.module, stats.initMs);
      if ((index + 1) % 25 === 0) console.error(`measured ${index + 1}/${catalog.tools.length} tools`);
    }
    // Each workflow's whole chain, through the same chain runner the pages and
    // the MCP server use, on its example inputs.
    const moduleOf = Object.fromEntries(catalog.tools.map((t) => [t.id, t.module]));
    const chains = [];
    for (const w of workflows) {
      const times = await page.evaluate(async ({ path, workflow, moduleOf, warmup, samples }) => {
        const { runChain } = await import(`/${path}/chain.mjs`);
        const invoke = async (id, input) => JSON.parse(await (await window.__benchmark.get(moduleOf[id])).module.invoke(id, JSON.stringify(input)));
        const once = async () => {
          const run = await runChain(workflow, {}, invoke);
          if (!run.ok) throw new Error(`${workflow.slug} fails on its example at step ${run.failed + 1}`);
        };
        for (let i = 0; i < warmup; i++) await once();
        const out = [];
        for (let i = 0; i < samples; i++) {
          const start = performance.now();
          await once();
          out.push(performance.now() - start);
        }
        window.__benchmark.durations.clear();
        return out;
      }, { path: benchPath, workflow: w, moduleOf, warmup: CHAIN_WARMUP, samples: CHAIN_SAMPLES });
      times.sort((a, b) => a - b);
      chains.push({ slug: w.slug, steps: w.steps.length, p50Ms: percentile(times, 0.5), p95Ms: percentile(times, 0.95), budgetMs: CHAIN_BUDGETS_MS[w.slug] ?? null });
    }
    const report = {
      profile: profile.version, host: 'chromium-main-thread', measurement: 'synchronous-abi', cpuTarget: profile.cpu.targetBenchmarkIndex, cpuSlowdown: cpu.rate, hostBenchmarkIndex: cpu.hostIndex,
      warmup: WARMUP, samples: SAMPLES,
      modules: [...modules].map(([name, initMs]) => ({ name, initMs })), tools, chains,
    };
    const baseline = option('--baseline');
    const rows = baseline ? compare(report, JSON.parse(readFileSync(baseline, 'utf8'))) : tools;
    console.log(`Chromium benchmark: profile ${profile.version}, ${cpu.rate}× CPU slowdown (host BenchmarkIndex ${cpu.hostIndex}, target ${profile.cpu.targetBenchmarkIndex}), ${WARMUP} warm-up and ${SAMPLES} measured calls per tool`);
    if (tools.length) console.log(table(rows));
    console.log(`\nWorkflow chains (${CHAIN_WARMUP} warm-up and ${CHAIN_SAMPLES} measured runs each)\n`);
    console.log('| Workflow | Steps | p50 ms | p95 ms | Budget |\n|---|---|---|---|---|');
    for (const c of chains) console.log(`| ${c.slug} | ${c.steps} | ${c.p50Ms.toFixed(2)} | ${c.p95Ms.toFixed(2)} | ${c.budgetMs ? `${c.budgetMs} ms` : ''} |`);
    const slowChains = chains.filter((c) => c.budgetMs && c.p95Ms > c.budgetMs);
    if (slowChains.length > 0) {
      console.error(`${slowChains.map((c) => `${c.slug} p95 ${c.p95Ms.toFixed(1)} ms over ${c.budgetMs} ms`).join('; ')}`);
      process.exitCode = 1;
    }
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
