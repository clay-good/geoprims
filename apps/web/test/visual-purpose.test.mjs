// Every visual earns its place (align-visuals-with-jobs): a page's map draws
// the reader's own inputs or answer, and a tool that declares a diagram has
// one. Driven from the catalog through the real core, with counts asserted so
// the checks cannot pass by reaching nothing.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { nodeHost } from '../../../packages/runtime/src/node.mjs';
import { buildLayers, mapsTool } from '../src/lib/map/layers.js';
import { DIAGRAM_TOOLS, diagram } from '../src/lib/diagrams.js';

const root = join(new URL('..', import.meta.url).pathname, '../..');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const host = nodeHost(join(root, 'dist/wasm'));
const invoke = async (id, input) => JSON.parse(await host.invoke(id, JSON.stringify(input)));
const primary = (t) => (t.examples.find((e) => e.id === t['x-primary-example']) ?? t.examples[0]).input;
const cells = {
  boundaries: (ids, grid = 'h3') =>
    Promise.all(ids.map(async (cell) => {
      const r = await invoke(`indexing.${grid}.cell-info`, { cell });
      return r.ok ? r.result.boundary.map((p) => [p.lon?.value ?? p.lon, p.lat?.value ?? p.lat]) : [];
    })),
  rings: (origin, k) =>
    Promise.all(Array.from({ length: k + 1 }, async (_, d) => {
      const r = await invoke('indexing.h3.grid-ring', { cell: origin, k: d });
      return r.ok ? r.result.cells.map((c) => c.cell) : [];
    })),
};

test('every page that carries the map draws something of the example on it', async () => {
  const empty = [];
  let reached = 0;
  for (const t of catalog.tools.filter(mapsTool)) {
    const ex = primary(t);
    const result = await invoke(t.id, ex);
    assert.ok(result.ok, `${t.id}: example fails`);
    const layers = await buildLayers(t, ex, result, (i) => invoke('navigation.geodesic.waypoints', i), cells);
    reached++;
    // A tool whose location is optional may leave it out of its example; the
    // canvas then hides itself, so that page shows no empty map either.
    const optional = !(t.inputs.required ?? []).includes('lat');
    if (!layers.length && !('lat' in ex) && 'lat' in t.inputs.properties && optional) continue;
    if (!layers.length) empty.push(t.id);
  }
  assert.deepEqual(empty, [], 'these pages would show a map with nothing of the reader on it');
  assert.ok(reached > 80, `${reached} map pages reached`);
});

test('pages whose coordinates are not on the earth carry no map', () => {
  // Plane survey coordinates (northing and easting on a local grid) and a
  // camera's footprint have no place on a globe: they get a diagram instead.
  for (const id of ['drone.photogrammetry.gsd', 'survey.cogo.forward', 'survey.cogo.area-by-coordinates', 'survey.cogo.traverse-closure']) {
    const t = catalog.tools.find((x) => x.id === id);
    assert.ok(t, id);
    assert.equal(mapsTool(t), false, `${id} carries a map`);
    assert.ok(DIAGRAM_TOOLS.includes(id), `${id} has no diagram`);
  }
});

test('a tool that declares a vector diagram draws one from its example', async () => {
  const declared = catalog.tools.filter((t) => (t.visualization ?? []).some((v) => v.kind === 'vector-diagram'));
  const missing = [];
  for (const t of declared) {
    const ex = primary(t);
    const d = diagram(t.id, ex, await invoke(t.id, ex));
    if (!d) missing.push(t.id);
  }
  assert.deepEqual(missing, [], 'declared a vector diagram but draws none');
  assert.ok(declared.length >= 20, `${declared.length} vector-diagram tools`);
});
