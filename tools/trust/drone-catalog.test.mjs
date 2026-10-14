// Drone-suite catalog progress against the 42-operation design inventory of
// add-drone-suite. The inventory stays in the spec until every planned
// operation is independently addressable.
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

const root = new URL('../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const drone = catalog.tools.filter((tool) => tool.domain === 'drone' && tool.composedOf.length === 0);
const byId = new Map(drone.map((tool) => [tool.id, tool]));

test('each photogrammetry step named in the inventory is an addressable drone operation', () => {
  const expected = new Map([
    ['drone.photogrammetry.footprint', ['image footprint calculator', 'photo ground coverage']],
    ['drone.photogrammetry.crop-factor', ['crop factor calculator', '35mm equivalent focal length']],
    ['drone.photogrammetry.trigger-distance', ['trigger distance calculator', 'distance between photos']],
    ['drone.photogrammetry.trigger-interval', ['seconds between photos', 'trigger interval for overlap']],
    ['drone.photogrammetry.line-spacing', ['flight line spacing calculator', 'distance between flight lines']],
    ['drone.photogrammetry.max-shutter', ['max shutter speed calculator', 'slowest shutter for drone mapping']],
    ['drone.photogrammetry.checkpoint-requirements', ['checkpoint requirements', 'minimum number of checkpoints']],
  ]);
  for (const [id, aliases] of expected) {
    const tool = byId.get(id);
    assert.ok(tool, `${id} is missing`);
    for (const alias of aliases) assert.ok(tool.aliases.includes(alias), `${id} is missing alias "${alias}"`);
    assert.ok(existsSync(join(root, `docs/derivations/${id}.md`)), `${id} needs a derivation note`);
    assert.ok(tool.vectorCount >= 20, `${id} has only ${tool.vectorCount} vectors`);
  }
});

test('each power step named in the inventory is an addressable drone operation', () => {
  const expected = new Map([
    ['drone.power.usable-energy', ['usable battery energy', 'battery reserve calculator']],
    ['drone.power.c-rate', ['C-rate calculator', 'battery discharge rate']],
    ['drone.power.disk-area', ['rotor disk area calculator', 'disk loading calculator']],
    ['drone.power.range', ['drone range calculator', 'how far can a drone fly']],
    ['drone.power.temperature-derating', ['cold battery derating', 'battery capacity in cold weather']],
  ]);
  for (const [id, aliases] of expected) {
    const tool = byId.get(id);
    assert.ok(tool, `${id} is missing`);
    for (const alias of aliases) assert.ok(tool.aliases.includes(alias), `${id} is missing alias "${alias}"`);
    assert.ok(existsSync(join(root, `docs/derivations/${id}.md`)), `${id} needs a derivation note`);
    assert.ok(tool.vectorCount >= 20, `${id} has only ${tool.vectorCount} vectors`);
  }
});

test('the drone catalog does not lose registered operations', () => {
  assert.ok(drone.length >= 48, `drone operation count regressed to ${drone.length}`);
});
