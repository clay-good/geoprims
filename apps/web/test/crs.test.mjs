// Coordinates in a projected system (web/io-formats, "Coordinate order and CRS
// safety"): a CSV the reader says is UTM or State Plane, and a legacy GeoJSON
// `crs` naming a WGS 84 UTM zone, converted by the core's own inverse tools;
// every other declared system refused.
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { crsFromName, datumNote, projectParsed, toGeographic } from '../src/lib/crs.mjs';
import { readGeoJson } from '../src/lib/import.mjs';
import { csvProjected, importReport, rowsFor } from '../src/lib/import-rows.mjs';
import { nodeHost } from '../../../packages/runtime/src/node.mjs';

const web = new URL('..', import.meta.url).pathname;
const host = nodeHost(join(web, '../../dist/wasm'));
const invokeBatch = (id, json) => host.invokeBatch(id, json);
const invoke = async (id, input) => JSON.parse(await host.invoke(id, JSON.stringify(input)));
const points = { type: 'array', items: { properties: { name: {}, lat: {}, lon: {} } }, maxItems: 100 };
const polygon = { type: 'array', items: { properties: { lat: {}, lon: {}, ring: {} } }, maxItems: 100 };
const near = (a, b, tol, what) => assert.ok(Math.abs(a - b) <= tol, `${what}: ${a} vs ${b}`);

test('declared systems: WGS 84 and its UTM zones are known; everything else is not', () => {
  assert.deepEqual(crsFromName('urn:ogc:def:crs:OGC:1.3:CRS84'), { kind: 'wgs84' });
  assert.deepEqual(crsFromName('EPSG:4326'), { kind: 'wgs84' });
  assert.deepEqual(crsFromName('urn:ogc:def:crs:EPSG::4326'), { kind: 'wgs84' });
  assert.equal(crsFromName('urn:ogc:def:crs:EPSG::32617').zone, 17);
  assert.equal(crsFromName('EPSG:32617').hemisphere, 'N');
  assert.equal(crsFromName('EPSG:32733').hemisphere, 'S');
  for (const other of ['EPSG:32661', 'EPSG:32600', 'EPSG:26917', 'EPSG:3857', 'EPSG:2272', 'EPSG:43260', 'nonsense']) {
    assert.equal(crsFromName(other), null, other);
  }
});

test('a GeoJSON declaring a WGS 84 UTM zone is converted by the core, and says so', async () => {
  // Three corners of a square in zone 17N, made by the core's forward projection.
  const corners = [[40.44, -80], [40.44, -79.99], [40.45, -79.99]];
  const utm = [];
  for (const [lat, lon] of corners) {
    const r = await invoke('geodesy.utm.forward', { lat, lon, zone: 17 });
    utm.push([r.result.easting.value, r.result.northing.value]);
  }
  const text = JSON.stringify({ type: 'Feature', crs: { type: 'name', properties: { name: 'urn:ogc:def:crs:EPSG::32617' } }, geometry: { type: 'Polygon', coordinates: [[...utm, utm[0]]] } });
  const parsed = readGeoJson(text);
  assert.ok(parsed.ok);
  assert.equal(parsed.crs.kind, 'utm');
  const converted = await projectParsed(parsed, invokeBatch);
  assert.ok(converted.ok, converted.message);
  const ring = converted.geometries[0].coordinates;
  // The parser winds the ring counter-clockwise; every corner comes back.
  for (const [lat, lon] of corners) assert.ok(ring.some(([x, y]) => Math.abs(x - lon) < 1e-9 && Math.abs(y - lat) < 1e-9), `${lat}, ${lon}`);
  const filled = rowsFor(polygon, converted);
  assert.ok(filled.ok);
  assert.match(importReport('parcel.geojson', 'geojson', converted, filled), /Converted from UTM zone 17N \(WGS 84\)/);
});

test('any other declared system is refused, naming what is read', () => {
  const parsed = readGeoJson(JSON.stringify({ type: 'Feature', crs: { type: 'name', properties: { name: 'EPSG:2272' } }, geometry: { type: 'Point', coordinates: [2690000, 250000] } }));
  assert.equal(parsed.ok, false);
  assert.match(parsed.message, /EPSG:2272.*WGS 84 UTM zones/);
});

test('a CSV of UTM eastings and northings fills the input, converted in one batch', async () => {
  const parsed = { format: 'csv', headers: ['id', 'easting', 'northing'], rows: [['P1', '586309.953', '4477770.428'], ['P2', '587000', '4478000']] };
  const filled = await csvProjected(points, parsed, 1, 2, { kind: 'utm', zone: 17, hemisphere: 'N' }, invokeBatch);
  assert.ok(filled.ok, filled.message);
  near(filled.rows[0].lat, 40.446111, 1e-8, 'lat');
  near(filled.rows[0].lon, -79.982222, 1e-8, 'lon');
  assert.equal(filled.rows[0].name, 'P1');
  assert.equal(filled.converted, 'UTM zone 17N (WGS 84)');
  assert.equal(filled.datum, '');
});

test('a State Plane CSV converts in the zone’s unit and says the latitudes are NAD83', async () => {
  const parsed = { format: 'csv', headers: ['E', 'N'], rows: [['1340000', '410000']] };
  const filled = await csvProjected(points, parsed, 0, 1, { kind: 'spcs', zone: '3702', unit: 'ftUS' }, invokeBatch);
  assert.ok(filled.ok, filled.message);
  const direct = await invoke('geodesy.spcs.spcs83-inverse', { zone: '3702', easting: '1340000', northing: '410000', unit: 'ftUS' });
  assert.equal(filled.rows[0].lat, Number(direct.result.lat.value.toFixed(9)));
  assert.equal(filled.datum, datumNote({ kind: 'spcs' }));
  assert.match(importReport('pts.csv', 'csv', { converted: filled.converted, datum: filled.datum }, filled), /State Plane zone 3702 \(NAD83\).*NAD83/);
});

test('a bad row or a bad zone is named, never half-imported', async () => {
  const utm = { kind: 'utm', zone: 17, hemisphere: 'N' };
  const text = await csvProjected(points, { headers: ['e', 'n'], rows: [['586309', '4477770'], ['abc', '1']] }, 0, 1, utm, invokeBatch);
  assert.equal(text.ok, false);
  assert.match(text.message, /^Line 3 is not a pair of numbers/);
  const zone = await csvProjected(points, { headers: ['e', 'n'], rows: [['1', '2']] }, 0, 1, { kind: 'spcs', zone: '9999' }, invokeBatch);
  assert.equal(zone.ok, false);
  assert.match(zone.message, /^Line 2 could not be converted from State Plane zone 9999 \(NAD83\): 9999 is not an SPCS83 zone/);
  const same = await csvProjected(points, { headers: ['e'], rows: [['1']] }, 0, 0, utm, invokeBatch);
  assert.match(same.message, /two different columns/);
  assert.deepEqual(await toGeographic(utm, [], invokeBatch), { ok: true, coords: [] });
});
