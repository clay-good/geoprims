import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';

const root = new URL('../..', import.meta.url).pathname;
const registry = JSON.parse(readFileSync(join(root, 'assets/registry.json'), 'utf8'));
const asset = (id) => registry.assets.find((item) => item.id === id);

function embeddedZoneNames(bytes) {
  assert.equal(bytes.subarray(0, 4).toString(), 'GPTZ');
  let at = 5 + bytes[4];
  const count = bytes.readUInt16LE(at);
  at += 4;
  const names = new Set();
  for (let i = 0; i < count; i += 1) {
    const length = bytes[at];
    names.add(bytes.subarray(at + 1, at + 1 + length).toString());
    at += length + 3;
  }
  return names;
}

test('the time-zone boundary pack preserves its archive and ODbL license', () => {
  const row = asset('tz-boundaries');
  const dir = join(root, 'assets/data/tz-boundaries', row.version);
  const archive = readFileSync(join(dir, 'timezones.geojson.zip'));
  const license = readFileSync(join(dir, 'DATA_LICENSE'), 'utf8');

  assert.deepEqual([...archive.subarray(0, 4)], [0x50, 0x4b, 0x03, 0x04]);
  assert.match(archive.subarray(-256).toString('latin1'), /combined\.json/);
  assert.match(license, /Open Database License \(ODbL\) v1\.0/);
  assert.equal(row.license, 'ODbL-1.0');
  assert.match(row.attribution, /OpenStreetMap contributors/);
});

test('every boundary identifier is understood by the bundled time-zone rules', () => {
  const boundaryNames = JSON.parse(readFileSync(join(root, 'assets/data/tz-boundaries/2026e/timezone-names.json'), 'utf8'));
  const ruleNames = embeddedZoneNames(readFileSync(join(root, 'core/crates/gp-time/data/tzdb.bin')));

  assert.ok(boundaryNames.length > 400, 'boundary name list is unexpectedly small');
  assert.deepEqual(boundaryNames.filter((name) => !ruleNames.has(name)), []);
});

test('the NGS ANTINFO pack identifies the pinned IGS20 composite', () => {
  const row = asset('ngs-antinfo');
  const text = readFileSync(join(root, 'assets/data/ngs-antinfo/IGS20_2437/ngs20.003'), 'utf8');

  assert.match(text.split('\n', 1)[0], /TYP:ABS SRC:IGS20_2437,NGS-Database/);
  assert.match(text.split('\n', 1)[0], /NGS-26\/09\/29=1165/);
  assert.equal(row.version, 'IGS20_2437');
  assert.match(row.attribution, /National Geodetic Survey/);
});

test('both practitioner datasets are explicit whole-file offline packs', () => {
  for (const id of ['tz-boundaries', 'ngs-antinfo']) {
    const row = asset(id);
    assert.equal(row.loadPolicy, 'on-demand');
    assert.equal(row.tiling, 'none');
    assert.ok(row.offlinePack);
    assert.ok(Object.values(row.files).reduce((sum, file) => sum + file.bytes, 0) > 0);
  }
});
