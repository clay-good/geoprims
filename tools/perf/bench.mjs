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
const REGRESSION_FLOOR_MS = 0.5;
const BUDGET_KINDS = new Set(['closed-form', 'iterative', 'data-proportional']);

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
  // The throttle is calibrated per host, so reports agree on the device they emulate, not the multiplier.
  if (report.cpuTarget !== baseline.cpuTarget) throw new Error('baseline emulates a different CPU target');
  if (report.samples < 1000 || baseline.samples < 1000) throw new Error('both reports need at least 1,000 measured calls per tool');
  const previous = new Map(baseline.tools.map((t) => [t.id, t]));
  return report.tools.map((row) => {
    const old = previous.get(row.id);
    const change = old?.p95Ms > 0 ? (row.p95Ms / old.p95Ms - 1) * 100 : null;
    const absoluteChange = old ? row.p95Ms - old.p95Ms : null;
    return {
      ...row,
      previousP95Ms: old?.p95Ms ?? null,
      changePercent: change,
      regression: change !== null && change > REGRESSION_PERCENT && absoluteChange > REGRESSION_FLOOR_MS,
    };
  });
}

function arrayInputs(tool) {
  const found = [];
  const visit = (schema, prefix = '') => {
    for (const [name, field] of Object.entries(schema?.properties ?? {})) {
      const path = `${prefix}${name}`;
      if (field.type === 'array') found.push(path);
      if (field.properties) visit(field, `${path}.`);
      if (field.items?.properties) visit(field.items, `${path}[].`);
    }
  };
  visit(tool.inputs);
  return found;
}

export function applyBudgets(report, budgets, catalog) {
  for (const field of ['profile', 'host', 'measurement']) {
    if (report[field] !== budgets[field]) throw new Error(`budgets have a different ${field}`);
  }
  const tools = new Map(catalog.tools.map((tool) => [tool.id, tool]));
  const declared = budgets.tools ?? {};
  const checkBudget = (id, budget) => {
    if (!BUDGET_KINDS.has(budget?.kind)) throw new Error(`${id}: unknown performance kind ${budget?.kind}`);
    if (!(typeof budget.p95Ms === 'number' && Number.isFinite(budget.p95Ms) && budget.p95Ms > 0)) {
      throw new Error(`${id}: p95Ms must be a positive number`);
    }
    if (budget.kind === 'data-proportional' && !budget.scalesWith) {
      throw new Error(`${id}: data-proportional budget must name scalesWith`);
    }
  };
  checkBudget('default', budgets.default);
  for (const [id, budget] of Object.entries(declared)) {
    if (!tools.has(id)) throw new Error(`${id}: performance budget names no catalog tool`);
    checkBudget(id, budget);
  }
  for (const tool of tools.values()) {
    const arrays = arrayInputs(tool);
    if (arrays.length && declared[tool.id]?.kind !== 'data-proportional') {
      throw new Error(`${tool.id}: array inputs ${arrays.join(', ')} need a data-proportional budget`);
    }
  }
  return report.tools.map((row) => {
    const budget = declared[row.id] ?? budgets.default;
    return { ...row, performanceKind: budget.kind, budgetMs: budget.p95Ms, overBudget: row.p95Ms > budget.p95Ms };
  });
}

export function table(rows) {
  return [
    '| Tool | p50 ms | p95 ms | Budget | Previous p95 ms | Change |',
    '|---|---:|---:|---:|---:|---:|',
    ...rows.map((r) => `| ${r.id} | ${r.p50Ms.toFixed(3)} | ${r.p95Ms.toFixed(3)} | ${r.budgetMs?.toFixed(3) ?? '—'} | ${r.previousP95Ms?.toFixed(3) ?? '—'} | ${r.changePercent === null || r.changePercent === undefined ? '—' : `${r.changePercent >= 0 ? '+' : ''}${r.changePercent.toFixed(1)}%`} |`),
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
