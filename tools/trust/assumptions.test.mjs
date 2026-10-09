// Every constant a tool uses that the caller does not supply is declared with
// its value, unit, and source (trust/citations, "Citation record per tool").
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { metaschemaProblems } from './metaschema.mjs';

const root = join(new URL('.', import.meta.url).pathname, '../..');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const ledger = JSON.parse(readFileSync(join(root, 'data/sources-ledger.json'), 'utf8')).sources;
const sourceIds = new Set(ledger.map((r) => r.id));
const byId = (id) => catalog.tools.find((t) => t.id === id);

/**
 * The tools whose answers rest on constants the caller never sees. The list
 * grows as constants are declared; it may not shrink, so a tool cannot quietly
 * stop saying what it assumed.
 */
const MUST_DECLARE = [
  'aviation.atmosphere.isa',
  'aviation.altimetry.pressure-altitude',
  'aviation.altimetry.density-altitude',
  'geodesy.utm.forward',
  'geodesy.utm.inverse',
  'geodesy.ups.forward',
  'geodesy.ups.inverse',
  'geodesy.grid-ref.mgrs-forward',
  'geodesy.grid-ref.mgrs-inverse',
  'geodesy.projection.web-mercator-forward',
  'geodesy.projection.web-mercator-inverse',
  'indexing.s2.covering',
  'indexing.s2.lat-lng-to-cell',
  'indexing.s2.cell-info',
  'navigation.route.cross-track',
  'geometry.area.polygon',
  'indexing.geohash.encode',
  'indexing.plus-code.encode',
  'drone.power.hover-power',
  'drone.power.max-payload',
  'drone.power.calibrate-hover',
  'aviation.airspeed.cas-to-tas',
  'aviation.airspeed.tas-to-cas',
  'aviation.altimetry.cold-temperature',
  'geodesy.magnetic.declination',
  'geodesy.magnetic.true-to-magnetic',
  'geodesy.magnetic.grivation',
  'time.sun.events',
  'indexing.tile.from-point',
  'indexing.tile.ground-resolution',
  'indexing.tile.cover',
  'aviation.altimetry.isa-temperature',
  'aviation.altimetry.true-altitude',
  'time.scale.julian-date',
  'aviation.performance.turn',
  'aviation.performance.pivotal-altitude',
  'aviation.airspeed.tat-sat',
  'survey.reduction.edm-correction',
  'time.scale.gps-week',
  'time.scale.gps-to-utc',
  'aviation.atmosphere.humidity',
  'raster.scale.reflectance',
  'drone.photogrammetry.gsd',
  'drone.photogrammetry.altitude-for-gsd',
  'drone.photogrammetry.trigger',
  'drone.mission.facade',
  'drone.sensors.thermal-footprint',
  'drone.sensors.dataset-size',
  'drone.sensors.lidar-plan',
  'navigation.los.horizon',
  'navigation.los.visibility',
  'navigation.los.dip',
  'navigation.los.fresnel',
  'raster.terrain.line-of-sight',
  'survey.gnss.dop',
];

test('the density-altitude tool says which gas constant it used', () => {
  // The scenario: R = 287.05287 J/(kg·K), cited to ICAO Doc 7488.
  const t = byId('aviation.altimetry.density-altitude');
  const r = (t['x-assumptions'] ?? []).find((a) => /gas constant/i.test(a.name));
  assert.ok(r, 'no gas constant declared');
  assert.equal(r.value, '287.05287');
  assert.equal(r.unit, 'J/(kg K)');
  assert.equal(r.source, 'icao-7488');
  const row = ledger.find((x) => x.id === r.source);
  assert.match(row.name, /ICAO Standard Atmosphere/);
});

test('the humidity model is declared too, since it changes the answer', () => {
  const t = byId('aviation.altimetry.density-altitude');
  const magnus = (t['x-assumptions'] ?? []).filter((a) => a.source === 'alduchov');
  assert.equal(magnus.length, 3, 'the Magnus coefficients are not all declared');
  assert.deepEqual(magnus.map((a) => a.value).sort(), ['17.625', '243.04', '6.1094'].sort());
});

test('the photogrammetry defaults name their published values', () => {
  const gsd = byId('drone.photogrammetry.gsd')['x-assumptions'];
  assert.deepEqual(gsd.map(({ value, unit, source }) => [value, unit, source]), [
    ['36', 'mm', 'wolf-photogrammetry'],
    ['24', 'mm', 'wolf-photogrammetry'],
  ]);

  const altitude = byId('drone.photogrammetry.altitude-for-gsd')['x-assumptions'];
  assert.ok(altitude.some((a) => a.value === '400' && a.unit === 'ft' && a.source === 'cfr-14-107'));

  const trigger = byId('drone.photogrammetry.trigger')['x-assumptions'];
  assert.deepEqual(
    trigger.filter((a) => a.source === 'pix4d-overlap').map((a) => [a.value, a.unit]),
    [['75', '%'], ['60', '%'], ['85', '%'], ['85', '%']],
  );
});

test('the facade and thermal defaults cite the guidance they apply', () => {
  const facade = byId('drone.mission.facade')['x-assumptions'];
  assert.deepEqual(facade.map(({ value, unit, source }) => [value, unit, source]), [
    ['75', '%', 'pix4d-overlap'],
    ['60', '%', 'pix4d-overlap'],
  ]);

  const thermal = byId('drone.sensors.thermal-footprint')['x-assumptions'];
  assert.deepEqual(thermal.map(({ value, unit, source }) => [value, unit, source]), [
    ['3', 'px', 'flir-3x3'],
  ]);
});

test('the sensing tools expose their published LAS and USGS tables', () => {
  const dataset = byId('drone.sensors.dataset-size')['x-assumptions'];
  assert.deepEqual(dataset.map(({ value, unit, source }) => [value, unit, source]), [
    ['375', 'bytes', 'asprs-las-1-4'],
    ['20, 28, 26, 34, 57, 63, 30, 36, 38, 59, 67', 'bytes', 'asprs-las-1-4'],
    ['6', '1', 'asprs-las-1-4'],
  ]);

  const lidar = byId('drone.sensors.lidar-plan')['x-assumptions'];
  assert.deepEqual(lidar.map(({ value, unit, source }) => [value, unit, source]), [
    ['8.0', 'pulses/m2', 'usgs-lbs'],
    ['2.0', 'pulses/m2', 'usgs-lbs'],
    ['0.5', 'pulses/m2', 'usgs-lbs'],
  ]);
});

test('every tool that rests on constants declares them', () => {
  const missing = MUST_DECLARE.filter((id) => !(byId(id)?.['x-assumptions'] ?? []).length);
  assert.deepEqual(missing, []);
});

test('every declared constant is well formed and cites a known source', () => {
  const problems = metaschemaProblems(catalog, sourceIds).filter((p) => p.includes('assumption'));
  assert.deepEqual(problems, []);
  let declared = 0;
  for (const t of catalog.tools) declared += (t['x-assumptions'] ?? []).length;
  assert.ok(declared >= 20, `only ${declared} constants declared`);
});

test('the gate bites', () => {
  const t = { id: 'fixture.assume', examples: [], inputs: { properties: {} }, outputs: { properties: {} } };
  assert.deepEqual(metaschemaProblems({ tools: [{ ...t, 'x-assumptions': [{ name: 'R', value: 287, unit: 'J/(kg K)', source: 'nope' }] }] }, sourceIds), [
    'fixture.assume: assumption R cites unknown source nope',
  ]);
  assert.deepEqual(metaschemaProblems({ tools: [{ ...t, 'x-assumptions': [{ name: 'R', unit: '1', source: 'icao-7488' }] }] }, sourceIds), [
    'fixture.assume: assumption R has no value',
  ]);
  assert.deepEqual(metaschemaProblems({ tools: [{ ...t, 'x-assumptions': [{ name: 'R', value: 1, unit: '1', source: 'icao-7488', note: 'x' }] }] }, sourceIds), [
    'fixture.assume: assumption R has no field note',
  ]);
});

test('the page and the agent see the same constants', () => {
  const html = readFileSync(join(root, 'apps/web/dist/aviation/altimetry/density-altitude/index.html'), 'utf8');
  for (const a of byId('aviation.altimetry.density-altitude')['x-assumptions']) {
    assert.ok(html.includes(a.name), `${a.name} is not on the page`);
    assert.ok(html.includes(a.value), `${a.value} is not on the page`);
  }
  assert.match(readFileSync(join(root, 'mcp/meta.mjs'), 'utf8'), /assumptions: m\['x-assumptions'\]/);
});
