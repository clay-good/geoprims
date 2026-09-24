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
