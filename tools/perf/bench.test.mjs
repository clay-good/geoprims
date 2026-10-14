import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { applyBudgets, compare, measure, percentile, table } from './bench.mjs';

test('benchmark warms up, then reports nearest-rank p50 and p95 from 1,000 calls', async () => {
  let calls = 0;
  let clock = 0;
  const invoke = async () => { calls++; return '{"ok":true}'; };
  const result = await measure(invoke, 'fixture.a.b', '{}', { now: () => clock++ });
  assert.equal(calls, 1050);
  assert.deepEqual(result, { p50Ms: 1, p95Ms: 1 });
  assert.equal(percentile([1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 0.95), 10);
});

test('a slowed tool fails the baseline comparison and appears in the output table', () => {
  const baseline = { profile: '1.0.0', host: 'node', samples: 1000, tools: [{ id: 'navigation.geodesic.inverse', p95Ms: 10 }] };
  const report = { profile: '1.0.0', host: 'node', samples: 1000, tools: [{ id: 'navigation.geodesic.inverse', p50Ms: 8, p95Ms: 14 }] };
  const [row] = compare(report, baseline);
  assert.equal(row.regression, true);
  assert.match(table([row]), /navigation\.geodesic\.inverse \| 8\.000 \| 14\.000 \| — \| 10\.000 \| \+40\.0%/);
  assert.equal(compare({ ...report, tools: [{ ...report.tools[0], p95Ms: 11 }] }, baseline)[0].regression, false);
  assert.equal(compare({ ...report, tools: [{ ...report.tools[0], p95Ms: 0.7 }] }, { ...baseline, tools: [{ id: 'navigation.geodesic.inverse', p95Ms: 0.5 }] })[0].regression, false);
  assert.throws(() => compare({ ...report, profile: '2.0.0' }, baseline), /different reference profile/);
  assert.throws(() => compare({ ...report, measurement: 'synchronous-abi' }, baseline), /different timing method/);
  assert.throws(() => compare({ ...report, cpuTarget: 800 }, baseline), /different CPU target/);
  assert.throws(() => compare({ ...report, samples: 999 }, baseline), /1,000 measured calls/);
  const [newTool] = compare({ ...report, tools: [{ id: 'new.tool.id', p50Ms: 1, p95Ms: 2 }] }, baseline);
  assert.equal(newTool.changePercent, null);
  assert.equal(newTool.regression, false);
});

test('a rise the whole run shares is the host, and a tool that rises past it is still a regression', () => {
  const ids = Array.from({ length: 40 }, (_, i) => `fixture.tool.t${i}`);
  const shape = { profile: '1.1.0', host: 'chromium', samples: 1000 };
  const baseline = { ...shape, tools: ids.map((id, i) => ({ id, p95Ms: 0.7 + (i % 5) })) };
  const shifted = (extra = () => 0) => ({
    ...shape,
    tools: baseline.tools.map((t, i) => ({ id: t.id, p50Ms: 0.1, p95Ms: t.p95Ms + 0.4 + (i % 3) * 0.1 + extra(t.id) })),
  });
  // Run 38490441839: every tool 0.4 to 0.6 ms later on a faster, harder-throttled runner.
  const sameBuild = compare(shifted(), baseline);
  assert.equal(sameBuild[0].hostShiftMs.toFixed(1), '0.5');
  assert.deepEqual(sameBuild.filter((r) => r.regression), []);
  // On that same runner, one tool that really slowed down is found.
  const slowed = compare(shifted((id) => (id === 'fixture.tool.t7' ? 2 : 0)), baseline);
  assert.deepEqual(slowed.filter((r) => r.regression).map((r) => r.id), ['fixture.tool.t7']);
  // A faster run is not turned into regressions for the tools that held still.
  const faster = { ...shape, tools: baseline.tools.map((t, i) => ({ id: t.id, p50Ms: 0.1, p95Ms: i === 3 ? t.p95Ms : t.p95Ms - 0.4 })) };
  assert.equal(compare(faster, baseline)[0].hostShiftMs, 0);
  assert.deepEqual(compare(faster, baseline).filter((r) => r.regression), []);
  // A handful of tools is too few to tell a host from a regression: no shift is assumed.
  const few = { ...shape, tools: shifted().tools.slice(0, 5) };
  assert.equal(compare(few, { ...baseline, tools: baseline.tools.slice(0, 5) })[0].hostShiftMs, 0);
});

// The Chromium benchmark is the only caller of applyBudgets on the real
// catalog, and it runs last in CI: survey.reduction.level-adjust reached main
// without a budget for its level book and failed there, 30 minutes in.
test('the built catalog and the checked-in budgets agree, before the benchmark runs', () => {
  const read = (path) => JSON.parse(readFileSync(new URL(`../../${path}`, import.meta.url), 'utf8'));
  const budgets = read('data/performance-budgets.json');
  // An empty report in the budgets' own profile: only the declarations are checked.
  const report = { profile: budgets.profile, host: budgets.host, measurement: budgets.measurement, tools: [] };
  assert.doesNotThrow(() => applyBudgets(report, budgets, read('dist/catalog/v1.json')));
});

test('every list tool declares a data-proportional p95 budget', () => {
  const catalog = {
    tools: [
      { id: 'fixture.closed', inputs: { properties: { value: { type: 'number' } } } },
      { id: 'fixture.list', inputs: { properties: { points: { type: 'array' } } } },
    ],
  };
  const report = {
    profile: '1.1.0', host: 'chromium-main-thread', measurement: 'synchronous-abi',
    tools: [
      { id: 'fixture.closed', p50Ms: 1, p95Ms: 1.5 },
      { id: 'fixture.list', p50Ms: 4, p95Ms: 12 },
    ],
  };
  const budgets = {
    profile: '1.1.0', host: 'chromium-main-thread', measurement: 'synchronous-abi',
    default: { kind: 'closed-form', p95Ms: 2 },
    tools: { 'fixture.list': { kind: 'data-proportional', p95Ms: 10, scalesWith: 'points' } },
  };
  const rows = applyBudgets(report, budgets, catalog);
  assert.deepEqual(rows.map((r) => [r.id, r.budgetMs, r.overBudget]), [
    ['fixture.closed', 2, false],
    ['fixture.list', 10, true],
  ]);
  assert.throws(
    () => applyBudgets(report, { ...budgets, tools: {} }, catalog),
    /fixture\.list: array inputs points need a data-proportional budget/,
  );
  assert.throws(
    () => applyBudgets(report, { ...budgets, tools: { missing: { kind: 'iterative', p95Ms: 10 } } }, catalog),
    /missing: performance budget names no catalog tool/,
  );
});
