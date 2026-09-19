// Map canvas math (web/map-canvas): projections round-trip, the globe hides
// its far side, framing unwraps the antimeridian, and the Natural Earth file
// decodes to the expected layers.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { decode, forward, frame, inverse, unwrap } from '../src/lib/map/projection.js';

const web = new URL('..', import.meta.url).pathname;

test('both projections invert their forward transform', () => {
  for (const mode of ['map', 'globe']) {
    const view = { mode, lon: -30, lat: 35, scale: 300, width: 800, height: 600 };
    for (const [lon, lat] of [[-30, 35], [-60, 50], [0, 10], [10, 60], [-45, -20]]) {
      const p = forward(view, lon, lat);
      assert.ok(p, `${mode} ${lon},${lat} visible`);
      const [l, q] = inverse(view, p[0], p[1]);
      assert.ok(Math.abs(l - lon) < 1e-9 && Math.abs(q - lat) < 1e-9, `${mode} ${lon},${lat} -> ${l},${q}`);
    }
  }
});

test('the globe hides the far side; the center is at the middle of the canvas', () => {
  const view = { mode: 'globe', lon: 0, lat: 0, scale: 250, width: 600, height: 600 };
  assert.equal(forward(view, 180, 0), null);
  assert.deepEqual(forward(view, 0, 0), [300, 300]);
  assert.equal(inverse(view, 0, 0), null, 'a corner is off the globe');
});

test('framing a set across the antimeridian centers on it, not on Greenwich', () => {
  const v = frame('map', [[170, 10], [-170, 12]], 800, 600);
  assert.ok(Math.abs(Math.abs(v.lon) - 180) < 1e-9, `${v.lon}`);
  assert.deepEqual(unwrap([[170, 0], [-170, 0], [-160, 0]]).map(([l]) => l), [170, 190, 200]);
});

test('Natural Earth 110m decodes to land, borders, and lakes', () => {
  const ne = JSON.parse(readFileSync(join(web, 'public/basemap/ne-110m.json'), 'utf8'));
  assert.match(ne.source, /Natural Earth 5\.1\.2/);
  const land = ne.land.map(decode);
  assert.ok(land.length > 100 && ne.borders.length > 100 && ne.lakes.length > 10);
  for (const ring of land) for (const [lon, lat] of ring) assert.ok(Math.abs(lon) <= 180.01 && Math.abs(lat) <= 90.01);
  // A ring around Australia's east coast reaches past 150° E.
  assert.ok(land.some((r) => r.some(([lon, lat]) => lon > 150 && lat < -30 && lat > -40)));
});

test('on the globe, far-side points of a filled ring land on the limb', async () => {
  const { forwardLimb } = await import('../src/lib/map/projection.js');
  const view = { mode: 'globe', lon: 0, lat: 0, scale: 100, width: 300, height: 300 };
  const p = forwardLimb(view, 150, 10);
  assert.ok(Math.abs(Math.hypot(p[0] - 150, p[1] - 150) - 100) < 1e-9, `${p}`);
  assert.deepEqual(forwardLimb(view, 0, 0), [150, 150]);
});

test('layers: a geodesic tool gets its line, a rhumb comparison, and both ends; polygons are densified', async () => {
  const { buildLayers, numberOf } = await import('../src/lib/map/layers.js');
  assert.equal(numberOf('40.6 deg'), 40.6);
  assert.equal(numberOf('abc'), null);
  const calls = [];
  const densify = async (input) => {
    calls.push(input);
    return { ok: true, result: { points: [{ lat: { value: input.lat1 }, lon: { value: input.lon1 } }, { lat: { value: input.lat2 }, lon: { value: input.lon2 } }] } };
  };
  const geo = { inputs: { properties: {} }, visualization: [{ kind: 'line-geodesic', map: [] }] };
  const layers = await buildLayers(geo, { lat1: 40, lon1: -74, lat2: 51, lon2: 0 }, { ok: true, result: {} }, densify);
  assert.deepEqual(layers.map((l) => `${l.kind}:${l.role}`), ['line:result', 'line:comparison', 'point:input', 'point:input']);
  assert.deepEqual(calls.map((c) => c.path), ['geodesic', 'rhumb']);
  const poly = { inputs: { properties: { polygon: { type: 'array', items: { properties: { lat: {}, lon: {}, ring: {} } } } } }, visualization: [{ kind: 'polygon', map: [] }] };
  calls.length = 0;
  const p = await buildLayers(poly, { polygon: [{ lat: 0, lon: 0 }, { lat: 0, lon: 1 }, { lat: 1, lon: 1 }] }, { ok: true, result: {} }, densify);
  assert.equal(calls.length, 3, 'one densification per edge');
  assert.equal(p[0].kind, 'polygon');
});
