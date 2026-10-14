// Geodesy-suite catalog progress against the 89-operation design inventory of
// add-geodesy-suite. The inventory stays in the spec until every planned
// operation is independently addressable.
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

const root = new URL('../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const geodesy = catalog.tools.filter((tool) => tool.domain === 'geodesy' && tool.composedOf.length === 0);
const byId = new Map(geodesy.map((tool) => [tool.id, tool]));

test('each parse, format, ellipsoid, and magnetic step named in the inventory is an addressable geodesy operation', () => {
  const expected = new Map([
    ['geodesy.parse.parse-dms', ['DMS to decimal degrees converter', 'degrees minutes seconds to decimal']],
    ['geodesy.parse.parse-ddm', ['DDM to decimal degrees converter', 'degrees decimal minutes to decimal']],
    ['geodesy.parse.format-dd', ['decimal degrees formatter', 'lat long to decimal degrees']],
    ['geodesy.parse.format-dms', ['DMS formatter', 'decimal degrees to degrees minutes seconds']],
    ['geodesy.parse.format-ddm', ['DDM formatter', 'GPS degrees and minutes']],
    ['geodesy.ellipsoid.meridian-arc', ['meridian arc length calculator', 'meridian distance']],
    ['geodesy.ellipsoid.degree-lengths', ['length of a degree of longitude', 'meters per arcsecond']],
    ['geodesy.magnetic.magnetic-to-true', ['magnetic to true', 'compass bearing to true bearing']],
  ]);
  for (const [id, aliases] of expected) {
    const tool = byId.get(id);
    assert.ok(tool, `${id} is missing`);
    for (const alias of aliases) assert.ok(tool.aliases.includes(alias), `${id} is missing alias "${alias}"`);
    assert.ok(existsSync(join(root, `docs/derivations/${id}.md`)), `${id} needs a derivation note`);
    assert.ok(tool.vectorCount >= 20, `${id} has only ${tool.vectorCount} vectors`);
  }
});

test('each local-frame and grid-figure step named in the inventory is an addressable geodesy operation', () => {
  const expected = new Map([
    ['geodesy.frame.aer-to-enu', ['AER to ENU', 'look angle to offsets']],
    ['geodesy.frame.enu-to-aer', ['ENU to AER', 'offsets to look angle']],
    ['geodesy.frame.enu-ned', ['ENU to NED', 'NED to ENU']],
    ['geodesy.frame.local-rotation-matrix', ['ECEF to ENU rotation matrix', 'ECEF to NED rotation matrix']],
    ['geodesy.projection.convergence', ['grid convergence calculator', 'meridian convergence angle']],
    ['geodesy.projection.scale-factor', ['grid scale factor calculator', 'point scale factor']],
  ]);
  for (const [id, aliases] of expected) {
    const tool = byId.get(id);
    assert.ok(tool, `${id} is missing`);
    for (const alias of aliases) assert.ok(tool.aliases.includes(alias), `${id} is missing alias "${alias}"`);
    assert.ok(existsSync(join(root, `docs/derivations/${id}.md`)), `${id} needs a derivation note`);
    assert.ok(tool.vectorCount >= 20, `${id} has only ${tool.vectorCount} vectors`);
  }
});

test('each direction of the height conversion is an addressable geodesy operation', () => {
  for (const id of ['geodesy.height.ellipsoidal-to-orthometric', 'geodesy.height.orthometric-to-ellipsoidal', 'geodesy.magnetic.uncertainty-zone']) {
    const tool = byId.get(id);
    assert.ok(tool, `${id} is missing`);
    assert.ok(existsSync(join(root, `docs/derivations/${id}.md`)), `${id} needs a derivation note`);
    assert.ok(tool.vectorCount >= 20, `${id} has only ${tool.vectorCount} vectors`);
  }
});

test('the geodesy catalog does not lose registered operations', () => {
  assert.ok(geodesy.length >= 86, `geodesy operation count regressed to ${geodesy.length}`);
});
