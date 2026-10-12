// Exporting the canvas (web/map-canvas, "Export"). The attribution, the
// caption, the GeoJSON, and the standalone SVG are pure, so each is checked
// here; the PNG itself is composed from them in the browser.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { attributionLines, BASEMAP_ATTRIBUTION, caption, layersGeoJson, mapSvg, svgWithFooter, wrap } from '../src/lib/canvas-export.mjs';
import { buildLayers } from '../src/lib/map/layers.js';
import { diagram } from '../src/lib/diagrams.js';

const web = new URL('..', import.meta.url).pathname;
const registry = JSON.parse(readFileSync(join(web, '../../assets/registry.json'), 'utf8'));
const tool = { id: 'navigation.geodesic.inverse', version: '1.0.0', inputs: { properties: {} }, visualization: [{ kind: 'line-geodesic', map: [] }] };

test('export with attribution: a view of DEM-derived layers carries the DEM attribution', () => {
  // The scenario's shape: a result whose meta names a DEM tile, and a registry
  // entry for it written the way every real entry is.
  const dem = { id: 'copernicus-dem-glo30', version: '2023-11', attribution: 'Copernicus DEM GLO-30, © DLR e.V. 2010-2014 and © Airbus 2014-2018, provided under COPERNICUS by the European Union and ESA.' };
  const lines = attributionLines({ ok: true, meta: { assets: [{ id: dem.id, version: dem.version }] } }, { assets: [...registry.assets, dem] });
  assert.deepEqual(lines, [BASEMAP_ATTRIBUTION, dem.attribution]);
});

test('attribution comes from the shipped registry, once per asset, base map first', () => {
  const geoid = registry.assets.find((a) => a.id === 'egm96-15');
  assert.ok(geoid?.attribution, 'the registry names who to credit');
  const result = { ok: true, meta: { assets: [{ id: 'egm96-15', version: geoid.version }, { id: 'egm96-15', version: geoid.version }, { id: 'not-in-registry' }] } };
  assert.deepEqual(attributionLines(result, registry), [BASEMAP_ATTRIBUTION, geoid.attribution]);
  assert.deepEqual(attributionLines(result, registry, { basemap: false }), [geoid.attribution]);
  assert.deepEqual(attributionLines({ ok: true, meta: { assets: [] } }, registry), [BASEMAP_ATTRIBUTION]);
  // Every shipped asset can be credited on an export.
  for (const a of registry.assets) assert.ok(a.attribution?.trim(), `${a.id}: no attribution`);
});

test('the caption is the result sentence and the tool that made it', () => {
  assert.equal(caption(tool, { summary: 'The distance is 5,570 km.' }), 'The distance is 5,570 km. — geoprims navigation.geodesic.inverse 1.0.0');
  assert.equal(caption(tool, {}), 'geoprims navigation.geodesic.inverse 1.0.0');
});

test('GeoJSON has every drawn layer, [lon, lat], with polygon rings closed', async () => {
  const densify = async (i) => ({ ok: true, result: { points: [{ lat: { value: i.lat1 }, lon: { value: i.lon1 } }, { lat: { value: i.lat2 }, lon: { value: i.lon2 } }] } });
  const layers = await buildLayers(tool, { lat1: 40, lon1: -74, lat2: 51, lon2: 0 }, { ok: true, result: {} }, densify);
  const fc = layersGeoJson(layers, tool);
  assert.equal(fc.type, 'FeatureCollection');
  assert.deepEqual(fc.features.map((f) => `${f.geometry.type}:${f.properties.role}`), ['LineString:result', 'LineString:comparison', 'Point:input', 'Point:input']);
  assert.deepEqual(fc.features[2].geometry.coordinates, [-74, 40], 'longitude first, per RFC 7946');
  assert.equal(fc.features[0].properties.tool, tool.id);

  const poly = layersGeoJson([{ kind: 'polygon', role: 'input', rings: [[[0, 0], [1, 0], [1, 1]], [[0.2, 0.2], [0.4, 0.2], [0.4, 0.4], [0.2, 0.2]]] }], tool);
  const [outer, hole] = poly.features[0].geometry.coordinates;
  assert.deepEqual(outer.at(-1), outer[0], 'an open ring is closed');
  assert.equal(hole.length, 4, 'a closed ring is not closed twice');
  for (const f of fc.features.concat(poly.features)) {
    const flat = JSON.stringify(f.geometry.coordinates).match(/-?\d+(\.\d+)?/g).map(Number);
    assert.ok(flat.every(Number.isFinite));
  }
});

test('a downloaded SVG stands alone: styles, background, caption, and attribution written in', () => {
  const dg = diagram('aviation.wind.heading-groundspeed', { course: '90 deg', tas: '120 kt' }, {
    ok: true,
    result: { heading: { value: 80.4, unit: 'deg' }, groundspeed: { value: 118.3, unit: 'kt' } },
  });
  assert.ok(dg?.markup, 'the wind triangle draws');
  const markup = dg.markup;
  const out = svgWithFooter(markup, 'Heading 080° <& "wind">', ['Line two'], { style: '.dg-accent{stroke:#c2410c}', background: '#fff', ink: '#333' });
  assert.match(out, /^<svg[^>]*viewBox="0 0 320 (\d+)"/);
  assert.ok(Number(/viewBox="0 0 320 (\d+)"/.exec(out)[1]) > 240, 'the footer strip adds height');
  assert.match(out, /<style>\.dg-accent\{stroke:#c2410c\}<\/style>/);
  assert.match(out, /Heading 080° &lt;&amp; &quot;wind&quot;&gt;/, 'the caption is escaped');
  assert.match(out, />Line two<\/text><\/svg>$/);
  assert.equal(out.match(/<svg\b/g).length, 1);
});

test('footer text wraps at spaces to the width, so no caption runs off the edge', () => {
  assert.deepEqual(wrap('one two three four', (l) => l.length <= 9), ['one two', 'three', 'four']);
  assert.deepEqual(wrap('unbreakable', (l) => l.length <= 3), ['unbreakable']);
  const long = 'Fly heading 81.7° for a groundspeed of 108.7 kt, a wind correction of 8.3° to the left. — geoprims aviation.wind.heading-groundspeed 1.0.0';
  const out = svgWithFooter('<svg viewBox="0 0 320 240" xmlns="http://www.w3.org/2000/svg"></svg>', long, ['Base map: Natural Earth (public domain)']);
  const texts = [...out.matchAll(/<text[^>]*>([^<]*)<\/text>/g)].map((m) => m[1]);
  assert.ok(texts.length >= 3, 'the long caption takes more than one line');
  for (const t of texts) assert.ok(t.length * 5.5 <= 304, `too long for the width: ${t}`);
  assert.equal(texts.join(' ').replace(' Base map: Natural Earth (public domain)', ''), long);
});

test('a 2D map exports as vectors with its base, layers, caption, and attribution', () => {
  const view = { mode: 'map', lon: 180, lat: 0, scale: 180, width: 640, height: 360 };
  const base = {
    land: [[[170, -10], [-170, -10], [-170, 10], [170, 10]]],
    lakes: [], states: [], borders: [[[179, -8], [-179, 8]]],
  };
  const layers = [
    { kind: 'line', role: 'result', points: [[175, 0], [-175, 0]], path: 'geodesic' },
    { kind: 'polygon', role: 'input', rings: [[[176, -3], [177, -3], [177, -2], [176, -2]]], grid: true, gridKind: 'h3' },
    { kind: 'polygon', role: 'input', rings: [[[178, -2], [-178, -2], [-178, 2], [178, 2]]] },
    { kind: 'point', role: 'result', points: [[180, 0]], label: 'P' },
  ];
  const colors = { bg: '#fff', surface: '#fff', land: '#ddd', line: '#555', graticule: '#eee', text: '#111', muted: '#777', accent: '#c40' };
  const out = mapSvg(view, base, layers, colors, 'Across the antimeridian.', [BASEMAP_ATTRIBUTION]);
  assert.match(out, /^<svg[^>]*viewBox="0 0 640 \d+"/);
  assert.match(out, /data-layer="land"/);
  assert.match(out, /data-layer="line"/);
  assert.match(out, /fill="none"[^>]+data-layer="grid"/, 'grid context exports as an outline');
  assert.match(out, /data-layer="polygon"/);
  assert.match(out, /data-layer="point"/);
  assert.match(out, />Across the antimeridian\.<\/text>/);
  assert.match(out, />Base map: Natural Earth \(public domain\)<\/text>/);
  assert.ok(!out.includes('<image'), 'the SVG is vector, not a wrapped raster');
  assert.ok(out.length < 500_000, 'the export stays practical');
});

test('SVG export is limited to the 2D modes the contract names', () => {
  assert.throws(() => mapSvg({ mode: 'globe' }, null, [], {}, '', []), /2D map views/);
});

test('SVG export reduces a 100,000-vertex track to what its pixels can show', () => {
  const points = Array.from({ length: 100_000 }, (_, i) => [-10 + i / 5_000, Math.sin(i / 2_000)]);
  const colors = { bg: '#fff', surface: '#fff', land: '#ddd', line: '#555', graticule: '#eee', text: '#111', muted: '#777', accent: '#c40' };
  const out = mapSvg(
    { mode: 'equirect', lon: 0, lat: 0, scale: 500, width: 800, height: 500 },
    null,
    [{ kind: 'line', role: 'result', points }],
    colors,
    '',
    [],
  );
  assert.ok(out.length < 100_000, `${out.length} bytes`);
});
