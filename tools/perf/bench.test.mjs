import assert from 'node:assert/strict';
import { test } from 'node:test';
import { compare, measure, percentile, table } from './bench.mjs';

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
  assert.match(table([row]), /navigation\.geodesic\.inverse \| 8\.000 \| 14\.000 \| 10\.000 \| \+40\.0%/);
  assert.equal(compare({ ...report, tools: [{ ...report.tools[0], p95Ms: 11 }] }, baseline)[0].regression, false);
  assert.throws(() => compare({ ...report, profile: '2.0.0' }, baseline), /different reference profile/);
  assert.throws(() => compare({ ...report, measurement: 'synchronous-abi' }, baseline), /different timing method/);
  assert.throws(() => compare({ ...report, cpuSlowdown: 4 }, baseline), /different CPU slowdown/);
  assert.throws(() => compare({ ...report, samples: 999 }, baseline), /1,000 measured calls/);
  const [newTool] = compare({ ...report, tools: [{ id: 'new.tool.id', p50Ms: 1, p95Ms: 2 }] }, baseline);
  assert.equal(newTool.changePercent, null);
  assert.equal(newTool.regression, false);
});
