// Spatial-indexing catalog progress against the design inventory of
// add-spatial-indexing-and-raster. The inventory stays in the spec until every
// planned operation is independently addressable.
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

const root = new URL('../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const indexing = catalog.tools.filter((tool) => tool.domain === 'indexing' && tool.composedOf.length === 0);
const byId = new Map(indexing.map((tool) => [tool.id, tool]));

// The h3 row of the inventory. Two operations were built under another ID.
const BUILT_AS = new Map([
  ['h3.latlng-to-cell', 'h3.lat-lng-to-cell'],
  ['h3.local-ij', 'h3.cell-to-local-ij'],
  ['h3.resolution-table', 'h3.resolution-chooser'],
]);
const H3 = 'latlng-to-cell cell-to-latlng cell-boundary resolution base-cell is-pentagon is-class-iii validate grid-disk grid-ring grid-path grid-distance local-ij parent children center-child child-position compact uncompact polygon-to-cells cells-to-polygon directed-edges vertexes cell-area edge-length resolution-table';

test('every operation in the h3 row of the inventory is built', () => {
  const planned = H3.split(' ').map((name) => `h3.${name}`);
  assert.equal(planned.length, 26);
  const missing = planned.filter((name) => !byId.has(`indexing.${BUILT_AS.get(name) ?? name}`));
  assert.deepEqual(missing, []);
});

test('each focused H3 operation has its derivation note and vectors', () => {
  for (const name of ['cell-to-latlng', 'cell-boundary', 'resolution', 'base-cell', 'is-pentagon', 'is-class-iii', 'validate', 'cell-area', 'grid-distance', 'center-child', 'child-position', 'directed-edges', 'vertexes', 'edge-length']) {
    const id = `indexing.h3.${name}`;
    const tool = byId.get(id);
    assert.ok(tool, `${id} is missing`);
    assert.ok(existsSync(join(root, `docs/derivations/${id}.md`)), `${id} needs a derivation note`);
    assert.ok(tool.vectorCount >= 20, `${id} has only ${tool.vectorCount} vectors`);
  }
});

// The other rows: two operations in each were built under another ID.
const ROWS = {
  s2: 'point-to-cell token inspect vertices hierarchy neighbors area cover',
  tile: 'latlon-to-tile tile-bounds xyz-tms tile-quadkey tile-hierarchy bbox-tiles ground-resolution',
  geohash: 'encode decode neighbors cover',
  'plus-code': 'encode decode shorten',
  a5: 'lon-lat-to-cell cell-boundary hierarchy',
};
const ROW_BUILT_AS = new Map([
  ['s2.point-to-cell', 's2.lat-lng-to-cell'], ['s2.inspect', 's2.cell-info'], ['s2.cover', 's2.covering'],
  ['tile.latlon-to-tile', 'tile.from-point'], ['tile.tile-bounds', 'tile.bounds'], ['tile.tile-hierarchy', 'tile.family'], ['tile.bbox-tiles', 'tile.cover'],
]);

test('every operation in the s2, tile, geohash, plus-code, and a5 rows of the inventory is built', () => {
  const planned = Object.entries(ROWS).flatMap(([group, names]) => names.split(' ').map((name) => `${group}.${name}`));
  assert.equal(planned.length, 25);
  const missing = planned.filter((name) => !byId.has(`indexing.${ROW_BUILT_AS.get(name) ?? name}`));
  assert.deepEqual(missing, []);
});

test('the indexing catalog does not lose registered operations', () => {
  assert.ok(indexing.length >= 55, `indexing operation count regressed to ${indexing.length}`);
});
