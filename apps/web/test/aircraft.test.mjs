// Aircraft profiles (aviation/fuel-and-loading, "Aircraft profiles saved
// locally", scenario "Profile export"): an exported profile, imported again,
// restores every table exactly, and the tool computes the same answer from
// the restored inputs as from the originals.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { blank, fileName, fromFile, PROFILE_FIELDS, saveTool, TABLES, toFile, valuesFor } from '../src/lib/aircraft.mjs';
import { nodeHost } from '../../../packages/runtime/src/node.mjs';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const host = nodeHost(join(root, 'dist/wasm'));
const byId = (id) => catalog.tools.find((t) => t.id === id);
const exampleOf = (t) => (t.examples.find((e) => e.id === t['x-primary-example']) ?? t.examples[0]).input;

test('every field a profile keeps is a real input of its tool, tables included', () => {
  for (const [id, fields] of Object.entries(PROFILE_FIELDS)) {
    const t = byId(id);
    assert.ok(t, `${id} is not in the catalog`);
    for (const f of fields) {
      assert.ok(t.inputs.properties[f], `${id} has no input ${f}`);
      assert.equal(t.inputs.properties[f].type === 'array', TABLES.has(f), `${id} ${f}: table or not`);
    }
  }
});

test('export and re-import restore every table, and the answer with it', async () => {
  let p = blank('N12345 C172S');
  const examples = {};
  for (const id of Object.keys(PROFILE_FIELDS)) {
    examples[id] = exampleOf(byId(id));
    p = saveTool(p, id, examples[id]);
  }
  // Tables the worked examples leave out still have to survive the trip.
  p = saveTool(p, 'aviation.airspeed.cas-to-tas', {
    ...examples['aviation.airspeed.cas-to-tas'],
    calibration: [{ indicated: 60, calibrated: 62 }, { indicated: 100, calibrated: 99 }, { indicated: 140, calibrated: 138 }],
  });
  const back = fromFile(toFile(p));
  assert.equal(back.ok, true, back.message);
  assert.deepEqual(back.profile, p);
  assert.ok(Object.keys(valuesFor(back.profile, 'aviation.loading.weight-balance')).includes('envelope'));
  for (const id of Object.keys(PROFILE_FIELDS)) {
    const kept = valuesFor(back.profile, id);
    const input = { ...examples[id], ...kept };
    if (id === 'aviation.airspeed.cas-to-tas') continue; // its calibration was changed above
    const [a, b] = await Promise.all([host.invoke(id, JSON.stringify(examples[id])), host.invoke(id, JSON.stringify(input))]);
    assert.equal(b, a, `${id} answers differently from the restored profile`);
  }
});

test('a profile keeps aircraft data and leaves flight data out', () => {
  const p = saveTool(blank('x'), 'aviation.airspeed.cas-to-tas', { airspeed: '120 kt', pressure_altitude: '8000 ft', vne: '163 kt' });
  assert.deepEqual(valuesFor(p, 'aviation.airspeed.cas-to-tas'), { vne: '163 kt' });
});

test('import refuses what is not a profile and drops what it does not keep', () => {
  const bad = (text, re) => {
    const r = fromFile(text);
    assert.equal(r.ok, false);
    assert.match(r.message, re);
  };
  bad('not json', /not JSON/);
  bad('{"type":"FeatureCollection"}', /not a geoprims aircraft profile/);
  bad(JSON.stringify({ format: 'geoprims-aircraft', version: 2, name: 'x', tools: {} }), /version 2/);
  bad(JSON.stringify({ format: 'geoprims-aircraft', version: 1, name: ' ', tools: {} }), /needs a name/);
  bad('x'.repeat(1_000_001), /too large/);
  const r = fromFile(JSON.stringify({
    format: 'geoprims-aircraft', version: 1, name: ' Club Archer ',
    tools: {
      'aviation.loading.weight-balance': { mac: 60, stations: [{ name: 'Front', weight: 340, arm: 80.5 }], lat: 40, envelope: 'oops' },
      'geodesy.utm.forward': { lat: 40 },
    },
  }));
  assert.equal(r.ok, true);
  assert.equal(r.profile.name, 'Club Archer');
  assert.deepEqual(r.profile.tools, { 'aviation.loading.weight-balance': { mac: 60, stations: [{ name: 'Front', weight: 340, arm: 80.5 }] } });
});

test('the download is named after the aircraft', () => {
  assert.equal(fileName(blank('N12345 C172S')), 'n12345-c172s.aircraft.json');
  assert.equal(fileName(blank('***')), 'aircraft.aircraft.json');
});
