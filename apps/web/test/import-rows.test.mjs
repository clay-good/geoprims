// From a file to a tool's list of points (web/io-formats, "Drop a GPX track").
// The readers give [lon, lat]; the form takes rows in its own columns. This is
// the one place the order of a pair is decided, so it is checked on each shape.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { importReport, rowsFor, rowsText } from '../src/lib/import-rows.mjs';
import { readFile } from '../src/lib/import.mjs';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const input = (id, name) => catalog.tools.find((t) => t.id === id).inputs.properties[name];

const KML = `<?xml version="1.0"?><kml><Document><Placemark><name>Field 7</name><Polygon><outerBoundaryIs><LinearRing><coordinates>-105.00,40.00,0 -104.99,40.00,0 -104.99,40.01,0 -105.00,40.01,0</coordinates></LinearRing></outerBoundaryIs></Polygon></Placemark></Document></kml>`;
const GPX = `<gpx version="1.1"><trk><name>Ridge</name><trkseg><trkpt lat="40.1" lon="-105.1"/><trkpt lat="40.2" lon="-105.2"/><trkpt lat="40.3" lon="-105.3"/></trkseg></trk></gpx>`;

test('a KML boundary fills a polygon input as open lat, lon rows', () => {
  const parsed = readFile('field7.kml', KML);
  const filled = rowsFor(input('geometry.area.polygon', 'polygon'), parsed);
  assert.ok(filled.ok, filled.message);
  assert.deepEqual(filled.rows, [
    { lat: 40, lon: -105 },
    { lat: 40, lon: -104.99 },
    { lat: 40.01, lon: -104.99 },
    { lat: 40.01, lon: -105 },
  ]);
  assert.equal(rowsText(input('geometry.area.polygon', 'polygon'), filled.rows), '40, -105\n40, -104.99\n40.01, -104.99\n40.01, -105');
  // The closing corner the input does not need is not reported as a fix.
  assert.doesNotMatch(importReport('field7.kml', parsed.format, parsed, filled), /ring closed/);
});

test('a GPX track fills a line input in order, by its name', () => {
  // The scenario: a GPX file with one track dropped on a polyline input.
  const filled = rowsFor(input('drone.mission.corridor', 'centerline'), readFile('ridge.gpx', GPX));
  assert.ok(filled.ok, filled.message);
  assert.deepEqual(filled.rows.map((r) => [r.lat, r.lon]), [[40.1, -105.1], [40.2, -105.2], [40.3, -105.3]]);
  assert.equal(filled.used, '“Ridge”');
});

test('a hole becomes ring 1, after the outer ring', () => {
  const geo = JSON.stringify({
    type: 'Polygon',
    coordinates: [
      [[0, 0], [4, 0], [4, 4], [0, 4], [0, 0]],
      [[1, 1], [1, 2], [2, 2], [2, 1], [1, 1]],
    ],
  });
  const filled = rowsFor(input('geometry.area.polygon', 'polygon'), readFile('a.geojson', geo));
  assert.ok(filled.ok);
  assert.equal(filled.rows.filter((r) => r.ring === undefined).length, 4);
  assert.equal(filled.rows.filter((r) => r.ring === 1).length, 4);
});

test('named points fill a waypoint list with their names', () => {
  const gpx = `<gpx version="1.1"><wpt lat="39.86" lon="-104.67"><name>KDEN</name></wpt><wpt lat="40.04" lon="-105.23"><name>KBDU</name></wpt></gpx>`;
  const filled = rowsFor(input('navigation.route.legs', 'waypoints'), readFile('route.gpx', gpx));
  assert.ok(filled.ok);
  assert.deepEqual(filled.rows, [{ name: 'KDEN', lat: 39.86, lon: -104.67 }, { name: 'KBDU', lat: 40.04, lon: -105.23 }]);
  assert.equal(rowsText(input('navigation.route.legs', 'waypoints'), filled.rows), 'KDEN, 39.86, -104.67\nKBDU, 40.04, -105.23');
});

test('a file with more points than the input takes is refused, never cut short', () => {
  const coords = Array.from({ length: 150 }, (_, i) => [-105 + i * 0.001, 40]);
  const filled = rowsFor(input('navigation.route.legs', 'waypoints'), readFile('long.geojson', JSON.stringify({ type: 'LineString', coordinates: coords })));
  assert.equal(filled.ok, false);
  assert.match(filled.message, /150 points; this input takes at most 100/);
});

test('an input without coordinates, or a file without geometry, says so', () => {
  assert.equal(rowsFor(input('survey.cogo.area-by-coordinates', 'points'), readFile('a.kml', KML)).ok, false);
  assert.equal(rowsFor(input('geometry.area.polygon', 'polygon'), readFile('a.csv', 'a,b\n1,2')).ok, false);
  assert.equal(rowsFor(input('geometry.area.polygon', 'polygon'), { ok: false, message: 'broken' }).message, 'broken');
});

test('the report names what was read, what was ignored, and what was fixed', () => {
  const kml = KML.replace('<Document>', '<Document><NetworkLink><Link><href>https://example.test/x.kml</href></Link></NetworkLink>');
  const parsed = readFile('field7.kml', kml);
  const filled = rowsFor(input('drone.mission.corridor', 'centerline'), parsed);
  const report = importReport('field7.kml', parsed.format, parsed, filled);
  assert.match(report, /^Read 5 points from field7\.kml \(KML\)/);
  assert.match(report, /Ignored, never fetched: .*NetworkLink/);
});

test('every tool with a list of points offers the import', () => {
  const problems = [];
  for (const t of catalog.tools) {
    for (const [name, schema] of Object.entries(t.inputs.properties)) {
      if (schema.type !== 'array' || !['lat', 'lon'].every((c) => c in (schema.items?.properties ?? {}))) continue;
      const html = readFileSync(join(web, 'dist', ...t.id.split('.'), 'index.html'), 'utf8');
      if (!/class="import-button"/.test(html)) problems.push(`${t.id}: ${name} offers no import`);
    }
  }
  assert.deepEqual(problems, []);
});

test('a CSV fills once its columns are chosen, and a swapped choice is caught', async () => {
  const { csvRows } = await import('../src/lib/import-rows.mjs');
  const legs = input('navigation.route.legs', 'waypoints');
  const parsed = readFile('airports.csv', 'name,latitude,longitude\nKDEN,-104.6731,39.8617\nKBDU,-105.2256,40.0394\n');
  assert.equal(parsed.swapped, true, 'the swap was not noticed');
  // Taken as labelled, the first row's latitude is -104.67: refused, and it says why.
  const wrong = csvRows(legs, parsed, 1, 2);
  assert.equal(wrong.ok, false);
  assert.match(wrong.message, /Line 2 has latitude -104\.6731, beyond ±90\. The columns may be the other way round\./);
  // Swapped, it fills, names and all.
  const right = csvRows(legs, parsed, 2, 1);
  assert.ok(right.ok, right.message);
  assert.deepEqual(right.rows, [{ name: 'KDEN', lat: 39.8617, lon: -104.6731 }, { name: 'KBDU', lat: 40.0394, lon: -105.2256 }]);
  // And the report does not call a warning a fix.
  assert.doesNotMatch(importReport('airports.csv', 'csv', parsed, right), /Fixed/);
});

test('a CSV row that is not a coordinate is named by its line', async () => {
  const { csvRows } = await import('../src/lib/import-rows.mjs');
  const parsed = readFile('a.csv', 'lat,lon\n40,-105\nnorth,west\n');
  const out = csvRows(input('drone.mission.corridor', 'centerline'), parsed, 0, 1);
  assert.equal(out.ok, false);
  assert.match(out.message, /Line 3 is not a pair of numbers/);
  assert.equal(csvRows(input('drone.mission.corridor', 'centerline'), parsed, 0, 0).ok, false);
});
