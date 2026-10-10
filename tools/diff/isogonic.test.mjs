// Isogonic lines against an independent magnetic model: every point the tool
// puts on a line is handed to pygeomag (a separate Python implementation of
// WMM2025), whose declination there must be the line's value to within what
// contouring a grid allows. Skipped where no Python with pygeomag is found.
import { spawnSync } from 'node:child_process';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { nodeHost } from '../../packages/runtime/src/node.mjs';

const root = new URL('../..', import.meta.url).pathname;
const host = nodeHost(join(root, 'dist/wasm'));
const python = ['python3', '/usr/bin/python3'].find((p) => spawnSync(p, ['-c', 'import pygeomag'], { stdio: 'ignore' }).status === 0);

const REGIONS = [
  { date: '2026-09-18', south: 37, west: -109, north: 41, east: -102, interval: '1 deg' },
  { date: '2026-09-18', south: 24, west: -125, north: 50, east: -66 },
  { date: '2027-03-01', south: 35, west: -10, north: 60, east: 30, interval: '1 deg' },
  { date: '2025-06-15', south: -45, west: 110, north: -10, east: 155 },
  { date: '2028-11-30', south: -35, west: 10, north: 5, east: 45, interval: '2 deg' },
  { date: '2026-01-01', south: 55, west: -170, north: 72, east: -130, interval: '5 deg' },
];

test('every point on an isogonic line has that declination in pygeomag', { skip: !python && 'no Python with pygeomag is installed' }, async () => {
  const cases = [];
  for (const input of [...REGIONS, { date: '2026-09-18' }]) {
    const env = JSON.parse(await host.invoke('geodesy.magnetic.isogonic', JSON.stringify(input)));
    assert.equal(env.ok, true, JSON.stringify(env.error));
    assert.ok(env.result.lines.length > 50, `${JSON.stringify(input)}: ${env.result.lines.length} points`);
    cases.push({ date: input.date, points: env.result.lines.map((p) => [p.level.value, p.lat.value, p.lon.value]) });
  }
  const run = spawnSync(python, [join(root, 'tools/diff/isogonic.py')], { input: JSON.stringify(cases), encoding: 'utf8', maxBuffer: 1 << 26 });
  assert.equal(run.status, 0, run.stderr);
  const errors = JSON.parse(run.stdout);
  // A region: the grid is fine, and a line is within 0.2 degrees of the model
  // everywhere (measured: 0.0002 to 0.12).
  for (const [i, [worst]] of errors.slice(0, REGIONS.length).entries()) {
    assert.ok(worst < 0.2, `${JSON.stringify(REGIONS[i])}: a line point is ${worst.toFixed(3)} degrees off`);
  }
  // The whole globe: a coarser grid, so half the points are within 0.05
  // degrees and 95% within 0.3, and the worst, beside the magnetic poles
  // where the field turns fastest, stay under 1 (measured: 0.012, 0.21, 0.79).
  const [worst, p95, median] = errors[REGIONS.length];
  assert.ok(median < 0.05 && p95 < 0.3 && worst < 1, `globe: median ${median.toFixed(3)}, p95 ${p95.toFixed(3)}, worst ${worst.toFixed(3)}`);
});
