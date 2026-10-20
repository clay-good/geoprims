// Equipment profiles (web/equipment-profiles): the file format and both
// import paths, the dropped-field report, the limits, the one storage key,
// and the move of aircraft profiles saved before kinds existed.
import { test, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

// The browser's storage, in memory; `blocked` makes every call throw, as a
// private window can.
const stored = new Map();
let blocked = false;
const guard = () => {
  if (blocked) throw new Error('SecurityError');
};
globalThis.localStorage = {
  getItem: (k) => (guard(), stored.has(k) ? stored.get(k) : null),
  setItem: (k, v) => (guard(), stored.set(k, String(v))),
  removeItem: (k) => (guard(), stored.delete(k)),
};

const P = await import('../src/lib/profiles.mjs');

beforeEach(() => {
  stored.clear();
  blocked = false;
});

const camera = () =>
  P.saveTool(P.blank('camera', 'Zenmuse P1', '2026-10-19'), 'drone.photogrammetry.gsd', {
    sensor_width: '35.9 mm',
    sensor_height: '24 mm',
    image_width: 8192,
    image_height: 5460,
    focal_length: '35 mm',
    height: '120 m',
  });

test('a profile keeps only its kind’s fields: equipment, not the job', () => {
  const p = camera();
  assert.deepEqual(P.valuesFor(p, 'drone.photogrammetry.gsd'), {
    sensor_width: '35.9 mm',
    sensor_height: '24 mm',
    image_width: 8192,
    image_height: 5460,
    focal_length: '35 mm',
  });
  assert.deepEqual(P.kindsFor('drone.photogrammetry.gsd'), ['camera']);
  assert.equal(P.takesProfile('geodesy.utm.forward'), false);
});

test('a camera saved on one tool fills the same fields on the kind\'s other tools', () => {
  const p = camera();
  assert.deepEqual(P.valuesFor(p, 'drone.photogrammetry.trigger'), {
    sensor_width: '35.9 mm',
    sensor_height: '24 mm',
    image_width: 8192,
    image_height: 5460,
    focal_length: '35 mm',
  });
  // A tool's own saved value comes first.
  const own = P.saveTool(p, 'drone.photogrammetry.footprint', { focal_length: '24 mm' });
  assert.equal(P.valuesFor(own, 'drone.photogrammetry.footprint').focal_length, '24 mm');
  assert.equal(P.valuesFor(own, 'drone.photogrammetry.trigger').focal_length, '35 mm');
});

test('export and import round-trip in the geoprims-profile format', () => {
  const p = camera();
  const text = P.toFile(p);
  assert.equal(JSON.parse(text).format, 'geoprims-profile');
  const back = P.fromFile(text);
  assert.equal(back.ok, true, back.message);
  assert.deepEqual(back.profile, p);
  assert.deepEqual(back.dropped, []);
  assert.equal(P.fileName(p), 'zenmuse-p1.camera.json');
});

test('an aircraft file from before kinds imports as an aircraft profile and re-exports in the new format', () => {
  const old = {
    format: 'geoprims-aircraft',
    version: 1,
    name: 'N12345 C172S',
    tools: { 'aviation.loading.weight-balance': { mac: 58.8, stations: [{ name: 'Front', arm: 37 }], envelope: [{ arm: 35, weight: 1500 }] } },
  };
  const r = P.fromFile(JSON.stringify(old));
  assert.equal(r.ok, true, r.message);
  assert.equal(r.profile.kind, 'aircraft');
  assert.deepEqual(r.profile.tools, old.tools);
  const again = JSON.parse(P.toFile(r.profile));
  assert.equal(again.format, 'geoprims-profile');
  assert.equal(again.kind, 'aircraft');
});

test('an unknown field is dropped and reported', () => {
  const text = JSON.stringify({
    format: 'geoprims-profile',
    version: 1,
    kind: 'camera',
    name: 'P1',
    savedOn: '2026-10-01',
    tools: {
      'drone.photogrammetry.gsd': { sensor_width: '35.9 mm', wind_rating: '12 m/s' },
      'drone.ops.wind-limit': { wind_rating: '12 m/s' },
    },
  });
  const r = P.fromFile(text);
  assert.equal(r.ok, true, r.message);
  assert.deepEqual(r.profile.tools, { 'drone.photogrammetry.gsd': { sensor_width: '35.9 mm' } });
  assert.deepEqual(r.dropped, ['drone.photogrammetry.gsd wind_rating', 'drone.ops.wind-limit']);
  assert.equal(r.profile.savedOn, '2026-10-01');
});

test('import refuses what is not a profile, saying why', () => {
  const bad = (text, re) => {
    const r = P.fromFile(text);
    assert.equal(r.ok, false);
    assert.match(r.message, re);
  };
  bad('x'.repeat(1_000_001), /too large/);
  bad('not json', /not JSON/);
  bad('{"type":"FeatureCollection"}', /not a geoprims equipment profile/);
  const base = { format: 'geoprims-profile', version: 1, kind: 'camera', tools: {} };
  bad(JSON.stringify({ ...base, name: '' }), /name of 1 to 60/);
  bad(JSON.stringify({ ...base, name: 'x'.repeat(61) }), /name of 1 to 60/);
  bad(JSON.stringify({ ...base, name: 'x', kind: 'boat' }), /no "boat" profiles/);
  bad(JSON.stringify({ ...base, name: 'x', version: 2 }), /version 2/);
});

test('every profile lives under one key, by kind and name; rename and remove', () => {
  assert.equal(P.put(camera()), 'saved');
  assert.deepEqual(Object.keys(JSON.parse(stored.get('gp-profiles'))), ['camera']);
  assert.equal(P.rename('camera', 'Zenmuse P1', 'P1'), true);
  assert.deepEqual(Object.keys(P.ofKind('camera')), ['P1']);
  assert.equal(P.ofKind('camera').P1.name, 'P1');
  assert.equal(P.rename('camera', 'P1', ' '), false);
  P.remove('camera', 'P1');
  assert.deepEqual(P.ofKind('camera'), {});
});

test('at most 50 profiles per kind; replacing one is always allowed', () => {
  for (let i = 0; i < P.MAX_PER_KIND; i++) assert.equal(P.put(P.blank('drone', `d${i}`, '2026-10-19')), 'saved');
  assert.equal(P.put(P.blank('drone', 'one more', '2026-10-19')), 'full');
  assert.equal(P.put(P.blank('drone', 'd7', '2026-10-19')), 'saved');
  assert.equal(P.put(P.blank('camera', 'other kind', '2026-10-19')), 'saved');
});

test('aircraft profiles saved before kinds move to the one key on first read', () => {
  const old = { format: 'geoprims-aircraft', version: 1, name: 'Club Archer', tools: { 'aviation.airspeed.tas-to-cas': { vne: '154 kt' } } };
  stored.set('gp-aircraft', JSON.stringify({ 'Club Archer': old }));
  const mine = P.ofKind('aircraft');
  assert.deepEqual(Object.keys(mine), ['Club Archer']);
  assert.deepEqual(mine['Club Archer'].tools, old.tools);
  assert.equal(stored.has('gp-aircraft'), false);
  assert.ok(JSON.parse(stored.get('gp-profiles')).aircraft['Club Archer']);
});

test('blocked storage keeps profiles for this page and says so', () => {
  blocked = true;
  assert.equal(P.put(camera()), 'page');
  assert.ok(P.ofKind('camera')['Zenmuse P1']);
});

test('shipped examples are read-only, fill tools, and are never stored or counted', () => {
  const cams = P.examplesOf('camera');
  assert.deepEqual(Object.keys(cams), ['1-inch 20 MP camera']);
  const ex = cams['1-inch 20 MP camera'];
  assert.equal(ex.example, true);
  assert.equal(P.valuesFor(ex, 'drone.photogrammetry.trigger-interval').min_interval, '2 s');
  assert.equal(P.valuesFor(ex, 'drone.photogrammetry.trigger-interval').focal_length, '8.8 mm');
  assert.equal(P.put(ex), 'example');
  assert.deepEqual(P.ofKind('camera'), {});
});

