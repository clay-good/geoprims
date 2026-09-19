// Command palette (web/command-palette): the ranking scenarios, the latency
// budget at 1,000 entries, and the palette's wiring on every page.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { nodeHost } from '../../../packages/runtime/src/node.mjs';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));

async function searcher(tools) {
  const m = await nodeHost(join(root, 'dist/wasm')).module('search');
  await m.callString('gp_search_load', JSON.stringify(tools));
  return async (query) => JSON.parse(await m.callString('gp_search', JSON.stringify({ query, limit: 8, includeExperimental: true }))).result.results.map((r) => r.id);
}

test('palette scenarios: typos and abbreviations', async () => {
  const top = await searcher(catalog.tools);
  assert.equal((await top('densty alt'))[0], 'aviation.altimetry.density-altitude');
  const tas = await top('tas');
  assert.ok(tas.slice(0, 2).every((id) => id.startsWith('aviation.airspeed.')), tas.join(', '));
});

test('p95 search time for 20 queries against 1,000 entries is within 16 ms', async () => {
  const tools = [];
  for (let i = 0; tools.length < 1000; i++) {
    const t = catalog.tools[i % catalog.tools.length];
    tools.push(i < catalog.tools.length ? t : { ...t, id: `${t.id}-copy${i}` });
  }
  const top = await searcher(tools);
  const queries = ['d', 'de', 'den', 'dens', 'densi', 'densty alt', 'tas', 'wca', 'magnetic declination', 'utm to lat lon', 'h3',
    'sun position at noon', 'knots to mph', 'ground sample distance', 'traverse closure compass rule', 'x', 'geoid',
    'state plane pennsylvania south', 'crosswind component runway 27', 'battery flight time'];
  const times = [];
  for (let round = 0; round < 3; round++) {
    for (const q of queries) {
      const t0 = performance.now();
      await top(q);
      times.push(performance.now() - t0);
    }
  }
  times.sort((a, b) => a - b);
  const p95 = times[Math.floor(times.length * 0.95)];
  assert.ok(p95 <= 16, `p95 ${p95.toFixed(2)} ms`);
});

test('every page offers the palette by button, / and Ctrl/Cmd+K', () => {
  for (const path of ['index.html', 'aviation/index.html', 'aviation/altimetry/density-altitude/index.html']) {
    const html = readFileSync(join(web, 'dist', path), 'utf8');
    assert.match(html, /<button type="button" class="palette-open" aria-keyshortcuts="\/ Control\+K Meta\+K">/, path);
  }
  const src = readFileSync(join(web, 'src/lib/palette.js'), 'utf8');
  for (const need of ['role="combobox"', 'role="listbox"', 'aria-activedescendant', 'aria-live="polite"', "restore?.focus?.()"]) {
    assert.ok(src.includes(need), need);
  }
});

test('paste-to-detect: H3, ambiguous geohash, coordinates, MGRS, and altimeter groups', async () => {
  const { detectValues } = await import('../src/lib/detect.js');
  const host = nodeHost(join(root, 'dist/wasm'));
  const search = await host.module('search');
  const link = await host.module('link');
  const tools = new Map(catalog.tools.map((t) => [t.id, t]));
  const ctx = {
    candidates: async (q) => JSON.parse(await search.callString('gp_detect', JSON.stringify({ query: q }))).result.candidates,
    run: async (id, input) => JSON.parse(await host.invoke(id, JSON.stringify(input))),
    encode: async (state) => JSON.parse(await link.callString('gp_link_encode', JSON.stringify(state))),
    tools,
  };
  const found = async (q) => (await detectValues(q, ctx)).result.found;

  const [h3] = await found('8928308280fffff');
  assert.equal(h3.label, 'H3 cell');
  assert.match(h3.summary, /resolution 9/);
  assert.deepEqual(h3.actions.map((a) => a.id).slice(0, 3), ['indexing.h3.cell-info', 'indexing.h3.grid-disk', 'indexing.h3.parent']);
  assert.equal(h3.actions[0].input.cell, '8928308280fffff');

  const ambiguous = await found('9q8yy');
  assert.equal(ambiguous[0].kind, 'geohash', 'geohash first');

  const [coords] = await found('40.4461, -79.9822');
  assert.equal(coords.kind, 'coordinates');
  const utm = coords.actions.find((a) => a.id === 'geodesy.utm.forward');
  assert.ok(Math.abs(utm.input.lat - 40.4461) < 1e-9 && Math.abs(utm.input.lon + 79.9822) < 1e-9);
  // The link opens the tool with those inputs.
  const fragment = utm.href.split('#')[1];
  const decoded = JSON.parse(await link.callString('gp_link_decode', fragment));
  assert.equal(decoded.result.state.i.lat, utm.input.lat);

  assert.equal((await found('18T WL 80669 23543'))[0]?.kind, 'coordinates', 'MGRS parses as coordinates');
  const [alt] = await found('A2992');
  assert.match(alt.summary, /1,013/);
  assert.equal(alt.actions[0].input.altimeter, 'A2992');
  assert.equal((await found('12/1137/2551')).map((f) => f.kind).join(), 'xyz', 'the coordinate reading of a tile path is dropped');
  assert.deepEqual(await found('density altitude'), []);
});

test('a question with numbers opens the tool filled in; ambiguous values become choices', async () => {
  const { placements, prefillActions } = await import('../src/lib/prefill.js');
  const host = nodeHost(join(root, 'dist/wasm'));
  const m = await host.module('search');
  await m.callString('gp_search_load', JSON.stringify(catalog.tools));
  const link = await host.module('link');
  const encode = async (state) => JSON.parse(await link.callString('gp_link_encode', JSON.stringify(state)));
  const decode = async (href) => JSON.parse(await link.callString('gp_link_decode', href.split('#')[1]));
  const tools = new Map(catalog.tools.map((t) => [t.id, t]));
  const top = async (query) => JSON.parse(await m.callString('gp_search', JSON.stringify({ query, limit: 8, includeExperimental: true }))).result.results[0];

  // natural-language-prefill "Density altitude from text".
  const da = await top('density altitude 5000 ft 30C 29.80');
  const [open] = await prefillActions(da, tools.get(da.id), encode);
  assert.equal(open.title, 'Open with these values');
  assert.equal(open.summary, 'field elevation 5000 ft, altimeter setting 29.80 inHg, outside air temperature 30 °C');
  assert.ok(open.href.startsWith('/aviation/altimetry/density-altitude/#'));
  assert.deepEqual((await decode(open.href)).result.state.i, da.prefill);

  // "Two temperatures": both placements are offered; nothing is guessed.
  const two = await top('density altitude 30 20 29.92 5000');
  const choices = await prefillActions(two, tools.get(two.id), encode);
  assert.equal(choices.length, 2);
  assert.match(choices[0].head, /^Which is which\? 30 and 20 could each be /);
  const placed = await Promise.all(choices.map(async (c) => (await decode(c.href)).result.state.i));
  assert.deepEqual(placed.map((i) => [i.temperature, i.dew_point]), [['30 degC', '20 degC'], ['20 degC', '30 degC']]);
  assert.ok(placed.every((i) => i.elevation === '5000 ft' && i.altimeter === '29.92 inHg'));

  assert.deepEqual(await prefillActions(await top('density altitude'), tools.get('aviation.altimetry.density-altitude'), encode), []);
  assert.equal(placements([{ value: '1', candidates: ['a', 'b', 'c'] }, { value: '2', candidates: ['a', 'b', 'c'] }, { value: '3', candidates: ['a', 'b', 'c'] }]).length, 6);
});
