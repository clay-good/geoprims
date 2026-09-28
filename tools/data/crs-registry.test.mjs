import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { angle, normalizeSpcs2022, requiredEpsgCodes } from './crs-registry.mjs';

const root = new URL('../..', import.meta.url).pathname;
const registryPath = join(root, 'assets/data/crs-registry/2026-08-27/crs-registry.json');
const spcsPath = join(root, 'assets/data/spcs2022-beta/2026-06-01/spcs2022-beta.json');
const registry = JSON.parse(readFileSync(registryPath));
const spcs = JSON.parse(readFileSync(spcsPath));

test('NGS angles and records normalize without losing their exact parameters', () => {
  assert.equal(angle(`29°19'57"N`), 29 + 19 / 60 + 57 / 3600);
  assert.equal(angle(`90°00'W`), -90);
  const rows = normalizeSpcs2022([{
    'Zone code': '001001', 'Zone abrv': 'GULF', 'Zone name': 'Gulf', 'Zone type': 'Special use',
    'Proj type': 'LC1', 'Origin latitude': `28°00'N`, 'Origin longitude west': `90°00'W`,
    'Projection origin scale': '0.999600', 'Skew azimuth (deg)': '', 'False northing (m)': '533,400',
    'False easting (m)': '1,600,200', 'Design by': 'NGS', 'Reference frame': 'NATRF2022',
  }], [{
    'Zone code': '001001', 'Min lat (deg)': '23.82', 'Min lon west (deg)': '-97.33',
    'Max lat (deg)': '30.25', 'Max lon west (deg)': '-81.17',
  }]);
  assert.deepEqual(rows[0].definition, {
    projection: 'LC1', originLatitude: 28, originLongitude: -90, originScale: 0.9996,
    falseEastingMeters: 1600200, falseNorthingMeters: 533400,
  });
});

test('the curated registry contains every CRS used by current tools', () => {
  const entries = new Map(registry.crs.map((entry) => [entry.id, entry]));
  assert.equal(entries.size, registry.crs.length);
  for (const code of requiredEpsgCodes()) {
    const entry = entries.get(`EPSG:${code}`);
    assert.ok(entry, `EPSG:${code} is missing`);
    assert.equal(entry.status, 'official');
    assert.equal(entry.definition.id.authority, 'EPSG');
    assert.equal(entry.definition.id.code, code);
    assert.equal(entry.bounds.length, 4);
  }
  assert.equal(requiredEpsgCodes().length, 253);
});

test('all 953 SPCS2022 zones have definitions, bounds, frames, and beta status', () => {
  assert.equal(spcs.zones.length, 953);
  assert.deepEqual(registry.sources.SPCS2022, spcs.source);
  const fromRegistry = registry.crs.filter((entry) => entry.id.startsWith('NGS:SPCS2022:'));
  assert.deepEqual(fromRegistry, spcs.zones);
  for (const zone of spcs.zones) {
    assert.match(zone.id, /^NGS:SPCS2022:\d{6}$/);
    assert.equal(zone.status, 'beta');
    assert.equal(zone.publishedAt, '2026-06-01');
    assert.ok(['NATRF2022', 'PATRF2022', 'MATRF2022', 'CATRF2022'].includes(zone.referenceFrame));
    assert.ok(['LC1', 'TM', 'OMC'].includes(zone.definition.projection));
    assert.equal(zone.bounds.length, 4);
    assert.ok(zone.bounds.every(Number.isFinite));
  }
});

test('the registry stays within its on-demand asset budget', () => {
  assert.ok(statSync(registryPath).size <= 1_500_000, `${statSync(registryPath).size} exceeds 1.5 MB`);
});
