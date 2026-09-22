// Canvas input (web/map-canvas, "Canvas input"): which points can be dragged,
// which one a press lands on, how precise a dragged coordinate is, and which
// point a click sets. The browser check drags waypoint B end to end.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { clickTarget, dragDegrees, handleAt, handlesOf } from '../src/lib/map/handles.js';
import { buildLayers } from '../src/lib/map/layers.js';
import { frame } from '../src/lib/map/projection.js';

const densify = async () => null;
const two = { inputs: { properties: { lat1: {}, lon1: {}, lat2: {}, lon2: {} } }, visualization: [{ kind: 'line-geodesic', map: [] }] };
const one = { inputs: { properties: { lat: {}, lon: {} } }, visualization: [] };

test('input points carry their fields; lines and results are not draggable', async () => {
  const layers = await buildLayers(two, { lat1: 40, lon1: -74, lat2: 51, lon2: 0 }, { ok: true, result: {} }, densify);
  assert.deepEqual(handlesOf(layers).map((h) => [h.label, h.field]), [['A', { lat: 'lat1', lon: 'lon1' }], ['B', { lat: 'lat2', lon: 'lon2' }]]);
  const single = await buildLayers(one, { lat: 40, lon: -105 }, { ok: true, result: {} }, densify);
  assert.deepEqual(handlesOf(single).map((h) => h.field), [{ lat: 'lat', lon: 'lon' }]);
  // A tool whose inputs are not those fields gets nothing to drag.
  const other = await buildLayers({ inputs: { properties: {} }, visualization: [] }, { lat: 40, lon: -105 }, { ok: true, result: {} }, densify);
  assert.deepEqual(handlesOf(other), []);
});

test('a press lands on the nearest point within reach, or on none', async () => {
  const layers = await buildLayers(two, { lat1: 40, lon1: -74, lat2: 51, lon2: 0 }, { ok: true, result: {} }, densify);
  const view = { ...frame('map', layers.flatMap((l) => l.points), 800, 400), width: 800, height: 400 };
  const hs = handlesOf(layers);
  const { forward } = await import('../src/lib/map/projection.js');
  const [bx, by] = forward(view, 0, 51);
  assert.equal(handleAt(hs, view, bx + 5, by - 5).label, 'B');
  assert.equal(handleAt(hs, view, bx + 40, by), null, 'too far from any point');
});

test('a dragged coordinate keeps the decimals a pixel resolves, and stays on the globe', () => {
  assert.deepEqual(dragDegrees(51.123456789, 0.987654321, 1000), { lat: '51.123', lon: '0.988' });
  assert.deepEqual(dragDegrees(51.123456789, 0.987654321, 10), { lat: '51.12346', lon: '0.98765' });
  assert.deepEqual(dragDegrees(51.123456789, 0.987654321, 0.001), { lat: '51.1234568', lon: '0.9876543' });
  assert.deepEqual(dragDegrees(91, 190, 1000), { lat: '90', lon: '-170' });
});

test('a click sets the point only on single-point tools', () => {
  assert.deepEqual(clickTarget(one), { lat: 'lat', lon: 'lon' });
  assert.equal(clickTarget(two), null);
});
