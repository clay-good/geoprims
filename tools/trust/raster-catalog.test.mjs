// Raster catalog progress against the design inventory of
// add-spatial-indexing-and-raster. The terrain row lists 18 operations; the
// ones that read a hosted DEM wait on the GLO-30 tiles.
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

const root = new URL('../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const byId = new Map(catalog.tools.map((tool) => [tool.id, tool]));

// Built so far. Still to come: elevation, elevation-hae, profile, and
// sampling (hosted DEM); viewshed (worker); and dem-info, which describes a
// raster file's metadata.
const TERRAIN_BUILT = 'slope aspect hillshade contours line-of-sight fresnel-terrain dem-stats roughness-tri tpi curvature dem-difference-volume highest-point';

test('the built operations of the terrain row are addressable by their inventory names', () => {
  const missing = TERRAIN_BUILT.split(' ').filter((name) => !byId.has(`raster.terrain.${name}`));
  assert.deepEqual(missing, []);
});

test('each focused terrain operation has its derivation note, vectors, and parent', () => {
  for (const [name, parent] of [['slope', 'slope-aspect'], ['aspect', 'slope-aspect'], ['hillshade', 'slope-aspect'], ['roughness-tri', 'ruggedness'], ['tpi', 'ruggedness'], ['fresnel-terrain', 'line-of-sight']]) {
    const id = `raster.terrain.${name}`;
    const tool = byId.get(id);
    assert.ok(existsSync(join(root, `docs/derivations/${id}.md`)), `${id} needs a derivation note`);
    assert.ok(tool.vectorCount >= 20, `${id} has only ${tool.vectorCount} vectors`);
    assert.ok(tool.related.some((r) => r.id === `raster.terrain.${parent}` && r.reason === 'parent'), `${id} should name its parent`);
  }
});
