#!/usr/bin/env node
// Node half of platform/verification's reference-profile benchmark.
import { readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { performance } from 'node:perf_hooks';
import { nodeHost } from '../../packages/runtime/src/node.mjs';

const root = fileURLToPath(new URL('../..', import.meta.url));
const WARMUP = 50;
const SAMPLES = 1000;
const REGRESSION_PERCENT = 20;

export function percentile(sorted, percent) {
  return sorted[Math.ceil(percent * sorted.length) - 1];
}

export async function measure(invoke, id, input, { warmup = WARMUP, samples = SAMPLES, now = () => performance.now() } = {}) {
  for (let i = 0; i < warmup; i++) await invoke(id, input);
  const times = [];
  for (let i = 0; i < samples; i++) {
    const start = now();
    const result = await invoke(id, input);
    const elapsed = now() - start;
    if (!JSON.parse(result).ok) throw new Error(`${id} returned an error during the benchmark`);
    times.push(elapsed);
  }
  times.sort((a, b) => a - b);
  return { p50Ms: percentile(times, 0.5), p95Ms: percentile(times, 0.95) };
}

export function compare(report, baseline) {
  if (report.profile !== baseline.profile || report.host !== baseline.host) {
    throw new Error('baseline has a different reference profile or host');
  }
  if (report.measurement !== baseline.measurement) throw new Error('baseline uses a different timing method');
  if (report.cpuSlowdown !== baseline.cpuSlowdown) throw new Error('baseline has a different CPU slowdown');
  if (report.samples < 1000 || baseline.samples < 1000) throw new Error('both reports need at least 1,000 measured calls per tool');
  const previous = new Map(baseline.tools.map((t) => [t.id, t]));
  return report.tools.map((row) => {
    const old = previous.get(row.id);
    const change = old?.p95Ms > 0 ? (row.p95Ms / old.p95Ms - 1) * 100 : null;
    return { ...row, previousP95Ms: old?.p95Ms ?? null, changePercent: change, regression: change !== null && change > REGRESSION_PERCENT };
  });
}

export function table(rows) {
  return [
    '| Tool | p50 ms | p95 ms | Previous p95 ms | Change |',
    '|---|---:|---:|---:|---:|',
    ...rows.map((r) => `| ${r.id} | ${r.p50Ms.toFixed(3)} | ${r.p95Ms.toFixed(3)} | ${r.previousP95Ms?.toFixed(3) ?? '—'} | ${r.changePercent === null || r.changePercent === undefined ? '—' : `${r.changePercent >= 0 ? '+' : ''}${r.changePercent.toFixed(1)}%`} |`),
  ].join('\n');
}

function option(name) {
  const at = process.argv.indexOf(name);
  if (at < 0) return null;
  if (!process.argv[at + 1] || process.argv[at + 1].startsWith('--')) throw new Error(`${name} needs a path`);
  return process.argv[at + 1];
}

async function main() {
  const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
  const profile = JSON.parse(readFileSync(join(root, 'data/reference-profile.json'), 'utf8'));
  const host = nodeHost(join(root, 'dist/wasm'));
  const tools = [];
  for (const tool of catalog.tools) {
    const example = tool.examples.find((e) => e.id === tool['x-primary-example']) ?? tool.examples[0];
    const input = JSON.stringify(example.input);
    const stats = await measure(host.invoke, tool.id, input);
    tools.push({ id: tool.id, ...stats });
  }
  const report = { profile: profile.version, host: 'node', measurement: 'host-invoke', warmup: WARMUP, samples: SAMPLES, tools };
  const baselinePath = option('--baseline');
  const rows = baselinePath ? compare(report, JSON.parse(readFileSync(baselinePath, 'utf8'))) : tools;
  console.log(`Node benchmark: profile ${profile.version}, ${WARMUP} warm-up and ${SAMPLES} measured invocations per tool`);
  console.log(table(rows));
  const output = option('--output');
  if (output) writeFileSync(output, JSON.stringify(report, null, 2) + '\n');
  const regressions = rows.filter((r) => r.regression);
  if (regressions.length > 0) {
    console.error(`${regressions.length} tool(s) exceed the ${REGRESSION_PERCENT}% p95 regression limit; see the table above`);
    process.exitCode = 1;
  }
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  main().catch((error) => { console.error(error.message); process.exitCode = 1; });
}
