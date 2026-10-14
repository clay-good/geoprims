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

test('each mission and operations step named in the inventory is an addressable drone operation', () => {
  const expected = new Map([
    ['drone.mission.crosshatch', ['double grid mission', 'crosshatch flight pattern']],
    ['drone.mission.geofence-check', ['geofence check', 'waypoints outside geofence']],
    ['drone.mission.pattern-stats', ['flight path length calculator', 'waypoint mission distance']],
    ['drone.ops.structure-altitude', ['structure exception altitude', 'tower inspection altitude limit']],
  ]);
  for (const [id, aliases] of expected) {
    const tool = byId.get(id);
    assert.ok(tool, `${id} is missing`);
    for (const alias of aliases) assert.ok(tool.aliases.includes(alias), `${id} is missing alias "${alias}"`);
    assert.ok(existsSync(join(root, `docs/derivations/${id}.md`)), `${id} needs a derivation note`);
    assert.ok(tool.vectorCount >= 20, `${id} has only ${tool.vectorCount} vectors`);
  }
});

// The 42-operation design inventory of add-drone-suite. Two operations were
// built under a shorter ID than the inventory's name, and terrain-following
// waits on the GLO-30 terrain tiles; both are listed so the count is honest.
const BUILT_AS = new Map([
  ['mission.facade-scan', 'mission.facade'],
  ['mission.pattern-export', 'mission.export'],
]);
const WAITING = new Set(['mission.terrain-following']);
const INVENTORY = {
  photogrammetry: 'gsd altitude-for-gsd footprint trigger-distance trigger-interval line-spacing motion-blur max-shutter oblique-gsd terrain-overlap image-count asprs-accuracy checkpoint-requirements crop-factor',
  mission: 'survey-grid crosshatch corridor orbit facade-scan geofence geofence-check terrain-following pattern-stats pattern-export',
  power: 'battery-energy usable-energy c-rate disk-area hover-power endurance range payload-impact max-payload rth-budget temperature-derating',
  ops: 'part107-altitude structure-altitude speed-check kinetic-energy easa-subcategory remote-id-reference part108-proposed-reference',
};

test('every operation in the 42-item design inventory is built, except the one that waits on terrain', () => {
  const planned = Object.entries(INVENTORY).flatMap(([group, names]) => names.split(' ').map((name) => `${group}.${name}`));
  assert.equal(planned.length, 42);
  const missing = planned.filter((name) => !byId.has(`drone.${BUILT_AS.get(name) ?? name}`));
  assert.deepEqual(missing, [...WAITING]);
});

test('the drone catalog does not lose registered operations', () => {
  assert.ok(drone.length >= 52, `drone operation count regressed to ${drone.length}`);
});
