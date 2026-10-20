// Equipment profile kinds (web/equipment-profiles, "Profile kinds are data",
// scenario "Field drift caught"): every kept field is a real, public,
// equipment input of its tool, and the gate names the kind, tool, and field
// when one is not.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { profileProblems } from './profiles.mjs';

const root = new URL('../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const profiles = JSON.parse(readFileSync(join(root, 'data/profiles.json'), 'utf8'));

test('the six kinds exist and every kept field passes the gate', () => {
  assert.deepEqual(Object.keys(profiles.kinds), ['aircraft', 'drone', 'camera', 'lidar', 'gnss-receiver', 'radio']);
  assert.deepEqual(profileProblems(profiles, catalog), []);
  assert.equal(profiles.format, 'geoprims-profile');
  assert.deepEqual(profiles.limits, { maxBytes: 1_000_000, maxName: 60, perKind: 50 });
});

test('a renamed field fails, naming the kind, tool, and field', () => {
  const drifted = structuredClone(profiles);
  drifted.kinds.camera.tools['drone.photogrammetry.gsd'].push('sensor_wdth');
  assert.deepEqual(profileProblems(drifted, catalog), ['camera: drone.photogrammetry.gsd has no input sensor_wdth']);
});

test('a job value fails even when the tool takes it', () => {
  const job = structuredClone(profiles);
  job.kinds.drone.tools['drone.mission.sorties'].push('lat', 'headwind');
  assert.deepEqual(profileProblems(job, catalog), [
    'drone: drone.mission.sorties lat cannot be kept: it describes the job (a place, a time, or a load)',
    'drone: drone.mission.sorties headwind cannot be kept: it describes the job (a place, a time, or a load)',
  ]);
  const wind = structuredClone(profiles);
  wind.kinds.drone.tools['drone.ops.wind-limit'].push('wind_speed');
  assert.deepEqual(profileProblems(wind, catalog), [
    'drone: drone.ops.wind-limit wind_speed cannot be kept: wind is a condition of the day, not of the equipment',
  ]);
});

test('a table must be declared as one', () => {
  const t = structuredClone(profiles);
  t.kinds.aircraft.tables = t.kinds.aircraft.tables.filter((f) => f !== 'envelope');
  assert.deepEqual(profileProblems(t, catalog), ['aircraft: aviation.loading.weight-balance envelope is a table but not listed in tables']);
});

test('shipped examples keep only fields their kind keeps, and match the tools\' worked examples', () => {
  assert.deepEqual(profiles.examples.map((e) => e.kind), ['camera', 'drone', 'gnss-receiver']);
  const bad = structuredClone(profiles);
  bad.examples[0].tools['drone.photogrammetry.gsd'].wind_rating = '12 m/s';
  assert.deepEqual(profileProblems(bad, catalog), ['example 1-inch 20 MP camera: camera keeps no wind_rating for drone.photogrammetry.gsd']);
  for (const ex of profiles.examples) {
    for (const [id, values] of Object.entries(ex.tools)) {
      const t = catalog.tools.find((x) => x.id === id);
      const input = (t.examples.find((e) => e.id === t['x-primary-example']) ?? t.examples[0]).input;
      for (const [f, v] of Object.entries(values)) assert.deepEqual(v, input[f], `${ex.name}: ${id} ${f}`);
    }
  }
});

