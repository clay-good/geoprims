// Exporting a result (web/io-formats, "Supported export formats"). Every
// format is a pure function of the result, so each one is checked here against
// a real computed answer — and every geographic format is read back and
// compared point for point, because an export nobody can open is not an export.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { csvCell, exportText, fileName, FORMATS, pairsIn, pointsOf, toSheet } from '../src/lib/export.mjs';
import { nodeHost } from '../../../packages/runtime/src/node.mjs';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const host = nodeHost(join(root, 'dist/wasm'));
const byId = (id) => catalog.tools.find((t) => t.id === id);
const ran = async (id) => {
  const t = byId(id);
  const args = (t.examples.find((e) => e.id === t['x-primary-example']) ?? t.examples[0]).input;
  return [t, args, JSON.parse(await host.invoke(id, JSON.stringify(args)))];
};

/** The coordinates in an exported file, in order, as [lon, lat] pairs. */
const readBack = {
  geojson: (text) => JSON.parse(text).features.map((f) => f.geometry.coordinates),
  kml: (text) => [...text.matchAll(/<coordinates>([^<]+)<\/coordinates>/g)].map((m) => m[1].split(',').slice(0, 2).map(Number)),
  gpx: (text) => [...text.matchAll(/<wpt lat="([^"]+)" lon="([^"]+)"/g)].map((m) => [Number(m[2]), Number(m[1])]),
  wkt: (text) => [...text.matchAll(/(-?[\d.]+) (-?[\d.]+)/g)].map((m) => [Number(m[1]), Number(m[2])]),
};

test('every geographic format round-trips the points it was given', async () => {
  let checked = 0;
  for (const id of ['navigation.geodesic.direct', 'navigation.geodesic.midpoint', 'drone.mission.orbit', 'indexing.geohash.decode']) {
    const [tool, args, result] = await ran(id);
    const points = pointsOf(tool, result);
    assert.ok(points.length, `${id}: no points to export`);
    for (const format of FORMATS.filter((f) => f.geographic)) {
      const text = exportText(format.id, { tool, args, result, display: result.display });
      const back = readBack[format.id](text);
      assert.equal(back.length, points.length, `${id} ${format.id}: ${back.length} points, expected ${points.length}`);
      back.forEach(([lon, lat], i) => {
        assert.ok(Math.abs(lat - points[i].lat) < 1e-9, `${id} ${format.id} point ${i} latitude`);
        assert.ok(Math.abs(lon - points[i].lon) < 1e-9, `${id} ${format.id} point ${i} longitude`);
      });
      checked += 1;
    }
  }
  assert.equal(checked, 16);
});

test('GeoJSON is lon, lat per RFC 7946, and says which tool made it', async () => {
  const [tool, args, result] = await ran('navigation.geodesic.midpoint');
  const geo = JSON.parse(exportText('geojson', { tool, args, result }));
  assert.equal(geo.type, 'FeatureCollection');
  const [lon, lat] = geo.features[0].geometry.coordinates;
  assert.ok(Math.abs(lat) <= 90 && Math.abs(lon) > 1, 'the pair looks swapped');
  assert.equal(geo.features[0].properties.tool, 'navigation.geodesic.midpoint');
});

test('a tool with no coordinates offers no geographic format', async () => {
  const [tool, args, result] = await ran('aviation.altimetry.density-altitude');
  assert.deepEqual(pointsOf(tool, result), []);
  assert.equal(exportText('wkt', { tool, args, result }), '');
  assert.deepEqual(JSON.parse(exportText('geojson', { tool, args, result })).features, []);
});

test('the calculation sheet carries what an audit file needs', async () => {
  // The scenario: a surveyor exports a traverse closure as a sheet.
  const [tool, args, result] = await ran('survey.cogo.traverse-closure');
  const sheet = toSheet(tool, args, result, { display: result.display, today: '2026-09-20T12:00:00Z' });
  assert.match(sheet, /^# /, 'no title');
  for (const heading of ['## Inputs', '## Results', '## Method', '## Sources', '## Provenance']) {
    assert.ok(sheet.includes(heading), `no ${heading}`);
  }
  assert.ok(sheet.includes(result.meta.model), 'the method is not stated');
  for (const r of result.meta.references) assert.ok(sheet.includes(r.locator), `no locator for ${r.title}`);
  assert.match(sheet, new RegExp(`geoprims ${tool.id.replace(/\./g, '\\.')} ${tool.version.replace(/\./g, '\\.')}`));
  assert.match(sheet, /core \d/);
  assert.match(sheet, /Made 2026-09-20T12:00:00Z/);
  assert.match(sheet, /not a legal survey determination/i);
  // Every value the card shows is in the sheet.
  for (const v of Object.values(result.display)) assert.ok(sheet.includes(String(v)), `${v} is missing`);
});

test('CSV quotes what would otherwise break a row', () => {
  assert.equal(csvCell('plain'), 'plain');
  assert.equal(csvCell('a,b'), '"a,b"');
  assert.equal(csvCell('say "hi"'), '"say ""hi"""');
  assert.equal(csvCell({ value: 12.5, unit: 'ft' }), '12.5');
  assert.equal(csvCell(null), '');
});

test('a list result becomes one CSV row per row', async () => {
  const [tool, args, result] = await ran('drone.mission.orbit');
  const csv = exportText('csv', { tool, args, result }).trim().split('\n');
  const rows = Object.values(result.result).find((v) => Array.isArray(v));
  assert.equal(csv.length, rows.length + 1, 'header plus one row each');
  assert.ok(csv[0].includes('lat') && csv[0].includes('lon'));
});

test('XML formats escape what would otherwise close a tag', async () => {
  const [tool, args, result] = await ran('navigation.geodesic.direct');
  const hostile = { ...tool, title: 'A <tool> & "friend"' };
  const kml = exportText('kml', { tool: hostile, args, result });
  assert.ok(!/<tool>/.test(kml), kml.slice(0, 200));
  assert.match(kml, /&lt;tool&gt; &amp; &quot;friend&quot;/);
});

test('a coordinate pair is found however it is named', () => {
  assert.deepEqual(pairsIn({ lat: 1, lon: 2 }), [['lat', 'lon']]);
  assert.deepEqual(pairsIn({ lat2: 1, lon2: 2 }), [['lat2', 'lon2']]);
  assert.deepEqual(pairsIn({ ref_lat: 1, ref_lon: 2 }), [['ref_lat', 'ref_lon']]);
  assert.deepEqual(pairsIn({ lat1: 1, lon1: 2, lat2: 3, lon2: 4 }), [['lat1', 'lon1'], ['lat2', 'lon2']]);
  // A latitude with no longitude beside it is not a point.
  assert.deepEqual(pairsIn({ lat: 1, distance: 2 }), []);
});

test('each format has a file name and a media type', () => {
  for (const f of FORMATS) {
    assert.ok(f.extension && f.type && f.label, f.id);
    assert.equal(fileName(byId('navigation.geodesic.direct'), f.id), `navigation.geodesic.direct.${f.extension}`);
  }
  assert.throws(() => exportText('nope', { tool: byId('navigation.geodesic.direct'), result: {} }), /no export format nope/);
});

test('the page offers the download', () => {
  const html = readFileSync(join(web, 'dist/navigation/geodesic/direct/index.html'), 'utf8');
  assert.match(html, /<button[^>]*>Download</);
});
