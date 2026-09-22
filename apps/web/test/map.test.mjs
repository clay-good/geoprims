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

test('layers: a grid cell (bbox) draws as its outline along parallels and meridians', async () => {
  const { buildLayers } = await import('../src/lib/map/layers.js');
  // The catalog's shape: map is an object.
  const tool = { inputs: { properties: {} }, visualization: [{ kind: 'bbox', map: { south: 'south', west: 'west', north: 'north', east: 'east' } }] };
  const deg = (value) => ({ value, unit: 'deg' });
  const result = { ok: true, result: { south: deg(40.4), west: deg(-80), north: deg(40.45), east: deg(-79.9) } };
  const [cell] = await buildLayers(tool, {}, result, async () => null);
  assert.equal(cell.kind, 'polygon');
  const ring = cell.rings[0];
  assert.equal(ring.length, 64);
  assert.ok(ring.every(([lon, lat]) => lon >= -80 && lon <= -79.9 && lat >= 40.4 && lat <= 40.45));
  // The MGRS bbox maps only a size: nothing to outline.
  const mgrs = { inputs: { properties: {} }, visualization: [{ kind: 'bbox', map: { size: 'square_size' } }] };
  assert.deepEqual(await buildLayers(mgrs, {}, { ok: true, result: { square_size: { value: 1 } } }, async () => null), []);
});

test('layers: every catalog tool builds its layers from its example without throwing', async () => {
  const { buildLayers } = await import('../src/lib/map/layers.js');
  const { nodeHost } = await import('../../../packages/runtime/src/node.mjs');
  const root = join(web, '../..');
  const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
  const host = nodeHost(join(root, 'dist/wasm'));
  let points = 0;
  for (const t of catalog.tools) {
    const ex = (t.examples.find((e) => e.id === t['x-primary-example']) ?? t.examples[0]).input;
    const result = JSON.parse(await host.invoke(t.id, JSON.stringify(ex)));
    const layers = await buildLayers(t, ex, result, async () => null);
    points += layers.filter((l) => l.role === 'result').length;
  }
  assert.ok(points > 20, `${points} result layers`);
});

test('a single point is framed with context around it, not zoomed to meters', () => {
  const v = frame('map', [[-105, 40]], 800, 400);
  // Pixels per radian: 12° across fits in 60% of 800 px, so under 2,300.
  assert.ok(v.scale < 2300 && v.scale > 1000, String(v.scale));
  const g = frame('globe', [[-105, 40]], 800, 400);
  assert.ok(g.scale <= (400 * 0.45) / 0.5 + 1e-9, String(g.scale));
});

test('h3 k-ring: k = 2 on a resolution 7 cell draws 19 cell outlines, the origin highlighted, fading by ring', async () => {
  const { buildLayers, cellIds } = await import('../src/lib/map/layers.js');
  const { nodeHost } = await import('../../../packages/runtime/src/node.mjs');
  const root = join(web, '../..');
  const host = nodeHost(join(root, 'dist/wasm'));
  const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
  const tool = catalog.tools.find((t) => t.id === 'indexing.h3.grid-disk');
  assert.equal(tool.visualization[0].kind, 'cell-set');
  const args = { cell: '872a8471effffff', k: 2 };
  const result = JSON.parse(await host.invoke(tool.id, JSON.stringify(args)));
  const batch = async (id, inputs) => JSON.parse(await host.invokeBatch(id, JSON.stringify(inputs)));
  const cells = {
    boundaries: async (ids) => (await batch('indexing.h3.cell-info', ids.map((cell) => ({ cell })))).map((r) => r.result.boundary.map((p) => [p.lon, p.lat])),
    rings: async (origin, k) => (await batch('indexing.h3.grid-ring', Array.from({ length: k + 1 }, (_, d) => ({ cell: origin, k: d })))).map((r) => r.result.cells.map((c) => c.cell)),
  };
  const layers = (await buildLayers(tool, args, result, async () => null, cells)).filter((l) => l.cell);
  assert.equal(layers.length, 19);
  assert.ok(layers.every((l) => l.rings[0].length === 6), 'hexagon outlines');
  const origin = layers.filter((l) => l.role === 'result');
  assert.deepEqual(origin.map((l) => l.cell), ['872a8471effffff']);
  assert.deepEqual([0, 1, 2].map((d) => layers.filter((l) => l.distance === d).length), [1, 6, 12]);
  const w = (d) => layers.find((l) => l.distance === d).weight;
  assert.ok(w(0) > w(1) && w(1) > w(2), 'intensity falls with ring distance');
  assert.deepEqual(cellIds({ result: { cell: 'abc' } }, 'cell'), ['abc']);
  assert.deepEqual(cellIds({ result: { cells: [{ cell: 'a' }, 'b'] } }, 'cells'), ['a', 'b']);
});

test('geohash, Plus Code, and tile cells draw as their bounds', async () => {
  const { buildLayers } = await import('../src/lib/map/layers.js');
  const { nodeHost } = await import('../../../packages/runtime/src/node.mjs');
  const root = join(web, '../..');
  const host = nodeHost(join(root, 'dist/wasm'));
  const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
  for (const id of ['indexing.geohash.decode', 'indexing.geohash.encode', 'indexing.plus-code.encode', 'indexing.plus-code.decode', 'indexing.tile.bounds']) {
    const t = catalog.tools.find((x) => x.id === id);
    const args = (t.examples.find((e) => e.id === t['x-primary-example']) ?? t.examples[0]).input;
    const r = JSON.parse(await host.invoke(id, JSON.stringify(args)));
    const cell = (await buildLayers(t, args, r, async () => null)).find((l) => l.kind === 'polygon');
    assert.ok(cell, `${id}: no cell outline`);
    const lons = cell.rings[0].map((p) => p[0]);
    const lats = cell.rings[0].map((p) => p[1]);
    assert.ok(Math.abs(Math.min(...lats) - r.result.south.value) < 1e-12 && Math.abs(Math.max(...lons) - r.result.east.value) < 1e-12, `${id}: outline is the core's bounds`);
  }
});

test('a computed polygon draws over its input, grouped by part and ring', async () => {
  const { buildLayers, outputRings } = await import('../src/lib/map/layers.js');
  const row = (lat, lon, part, ring) => ({ lat: { value: lat }, lon: { value: lon }, part, ring });
  const result = { ok: true, result: { boundary: [
    row(0, 0, 0, 0), row(0, 1, 0, 0), row(1, 1, 0, 0),
    row(0.2, 0.5, 0, 1), row(0.4, 0.6, 0, 1), row(0.4, 0.5, 0, 1),
    row(5, 5, 1, 0), row(5, 6, 1, 0), row(6, 6, 1, 0),
  ] } };
  assert.equal(outputRings(result, 'boundary').length, 3);
  const tool = {
    visualization: [{ kind: 'polygon', map: [['rings', 'boundary']] }],
    inputs: { properties: { vertices: { type: 'array', items: { properties: { lat: {}, lon: {} } } } } },
  };
  const args = { vertices: [{ lat: 0.1, lon: 0.1 }, { lat: 0.1, lon: 0.9 }, { lat: 0.8, lon: 0.9 }] };
  const layers = await buildLayers(tool, args, result, async () => null);
  assert.deepEqual(layers.map((l) => [l.kind, l.role, l.rings.length]), [['polygon', 'input', 1], ['polygon', 'result', 3]]);
});

test('a flight path draws from output waypoints, flattening a corridor’s lines', async () => {
  const { buildLayers, outputPath } = await import('../src/lib/map/layers.js');
  const wp = (lat, lon) => ({ lat: { value: lat, unit: 'deg' }, lon: { value: lon, unit: 'deg' } });
  const corridor = { ok: true, result: { lines: [{ offset: { value: -50 }, waypoints: [wp(0, 0), wp(0, 1)] }, { offset: { value: 50 }, waypoints: [wp(1, 1), wp(1, 0)] }] } };
  assert.deepEqual(outputPath(corridor, 'lines'), [[0, 0], [1, 0], [1, 1], [0, 1]]);
  const tool = {
    visualization: [{ kind: 'line-geodesic', map: [['path', 'lines']] }],
    inputs: { properties: { centerline: { type: 'array', items: { properties: { lat: {}, lon: {} } } } } },
  };
  const args = { centerline: [{ lat: 0.5, lon: 0 }, { lat: 0.5, lon: 1 }] };
  const layers = await buildLayers(tool, args, corridor, async () => null);
  const path = layers.find((l) => l.kind === 'line' && l.role === 'result');
  assert.equal(path.points.length, 4);
  assert.ok(path.arrows && path.stops);
  assert.ok(layers.some((l) => l.kind === 'point' && l.label === 'Start'));
  assert.ok(layers.some((l) => l.kind === 'line' && l.role === 'input' && l.points.length === 2));
});

test('layers: a route draws its legs in order, once, with a start marker', async () => {
  // add-navigation-and-geometry 2.8: the route the legs table describes is
  // drawn on the map, and the waypoints are not laid down twice.
  const { buildLayers } = await import('../src/lib/map/layers.js');
  const { nodeHost } = await import('../../../packages/runtime/src/node.mjs');
  const catalog = JSON.parse(readFileSync(join(web, '../../dist/catalog/v1.json'), 'utf8'));
  const host = nodeHost(join(web, '../../dist/wasm'));
  const tool = catalog.tools.find((t) => t.id === 'navigation.route.legs');
  const args = tool.examples[0].input;
  const result = JSON.parse(await host.invoke(tool.id, JSON.stringify(args)));
  const layers = await buildLayers(tool, args, result, null, null);
  const routes = layers.filter((l) => l.kind === 'line');
  assert.equal(routes.length, 1, 'the route is drawn once, not as input and result both');
  const [route] = routes;
  assert.equal(route.role, 'result');
  assert.equal(route.points.length, args.waypoints.length, 'a point per waypoint');
  for (const [i, w] of args.waypoints.entries()) {
    assert.ok(Math.abs(route.points[i][0] - w.lon) < 1e-9 && Math.abs(route.points[i][1] - w.lat) < 1e-9, `waypoint ${i} out of order`);
  }
  assert.ok(route.arrows, 'the legs carry their direction');
  const start = layers.find((l) => l.kind === 'point' && l.label === 'Start');
  assert.deepEqual(start.points[0], [args.waypoints[0].lon, args.waypoints[0].lat]);
});

test('h3 pentagon: a ring around a pentagon draws five neighbours, not six', async () => {
  // add-spatial-indexing-and-raster 1.4: the twelve pentagons are the cases
  // that break a hexagon assumption, so the drawing is pinned on one.
  const { buildLayers } = await import('../src/lib/map/layers.js');
  const { nodeHost } = await import('../../../packages/runtime/src/node.mjs');
  const root = join(web, '../..');
  const host = nodeHost(join(root, 'dist/wasm'));
  const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
  const tool = catalog.tools.find((t) => t.id === 'indexing.h3.grid-disk');
  const args = { cell: '85080003fffffff', k: 1 };
  const result = JSON.parse(await host.invoke(tool.id, JSON.stringify(args)));
  assert.equal(result.result.count, 6, 'a pentagon has five neighbours, so k=1 is six cells');
  assert.ok(result.meta.warnings.some((w) => w.code === 'PENTAGON_DISTORTION'), 'the distortion is flagged');
  const batch = async (id, inputs) => JSON.parse(await host.invokeBatch(id, JSON.stringify(inputs)));
  const cells = {
    boundaries: async (ids) => (await batch('indexing.h3.cell-info', ids.map((cell) => ({ cell })))).map((r) => r.result.boundary.map((p) => [p.lon, p.lat])),
    rings: async (origin, k) => (await batch('indexing.h3.grid-ring', Array.from({ length: k + 1 }, (_, d) => ({ cell: origin, k: d })))).map((r) => r.result.cells.map((c) => c.cell)),
  };
  const layers = (await buildLayers(tool, args, result, async () => null, cells)).filter((l) => l.cell);
  assert.equal(layers.length, 6, 'six outlines drawn');
  assert.deepEqual([0, 1].map((d) => layers.filter((l) => l.distance === d).length), [1, 5], 'one origin, five around it');
  // The pentagon itself is drawn with its own corner count, not a hexagon's.
  // Resolution 5 is Class III, where H3 gives a pentagon ten boundary
  // vertices: its five corners with a distortion vertex between each pair.
  const origin = layers.find((l) => l.role === 'result');
  assert.equal(origin.cell, args.cell);
  assert.equal(origin.rings[0].length, 10, 'the Class III pentagon outline');
  assert.ok(layers.filter((l) => l.distance === 1).every((l) => l.rings[0].length >= 5), 'its neighbours still close');
  // At a Class II resolution the same pentagon is drawn with five corners.
  const parent = JSON.parse(await host.invoke('indexing.h3.parent', JSON.stringify({ cell: args.cell, resolution: 4 })));
  const info = JSON.parse(await host.invoke('indexing.h3.cell-info', JSON.stringify({ cell: parent.result.parent })));
  assert.equal(info.result.pentagon, 'yes');
  assert.equal(info.result.boundary.length, 5, 'the Class II pentagon outline');
});
