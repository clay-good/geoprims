// The coordinate field (web/app-shell, "Schema-driven input forms"). A tool
// with a latitude and a longitude takes a coordinate however the reader has
// it, and says what it read before anything is computed.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { degrees, pairOf, readCoordinate } from '../src/lib/coordinate.mjs';
import { nodeHost } from '../../../packages/runtime/src/node.mjs';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const host = nodeHost(join(root, 'dist/wasm'));
const search = await host.module('search');
const ctx = {
  candidates: async (q) => JSON.parse(await search.callString('gp_detect', JSON.stringify({ query: q }))).result.candidates,
  run: async (id, input) => JSON.parse(await host.invoke(id, JSON.stringify(input))),
};
const byId = (id) => catalog.tools.find((t) => t.id === id);

test('a coordinate in degrees, minutes and seconds is read and shown', async () => {
  // The scenario: 40°26'46"N 79°58'56"W reads as 40.446111°, -79.982222°.
  const read = await readCoordinate('40°26\'46"N 79°58\'56"W', ctx);
  assert.ok(read.ok, read.message);
  assert.equal(degrees(read.lat), '40.4461111');
  assert.equal(degrees(read.lon), '-79.9822222');
  assert.equal(read.notation, 'DMS');
  assert.equal(read.ambiguous, null);
  assert.match(read.message, /Read as 40\.4461111°, -79\.9822222° \(DMS\)/);
});

test('a bare pair says which way round it was read, and offers the other', async () => {
  // The scenario: 40.4461 -79.9822 with no hemisphere or label.
  const read = await readCoordinate('40.4461 -79.9822', ctx);
  assert.ok(read.ok);
  assert.equal(read.message, 'Read as lat, lon: 40.4461°, -79.9822°');
  assert.deepEqual(read.ambiguous, { lat: -79.9822, lon: 40.4461 });
});

test('a pair that cannot be swapped offers no swap', async () => {
  // A longitude beyond ±90 cannot be a latitude, so there is no other reading.
  const read = await readCoordinate('40.4461 -179.9822', ctx);
  assert.ok(read.ok);
  assert.equal(read.ambiguous, null);
});

test('the notations a reader arrives with all work', async () => {
  for (const [text, lat, lon] of [
    ['9q8yyk8yuv', 37.7749309, -122.4194151],
    ['849VCWC8+R9', 37.4220625, -122.0840625],
    ['40 26 46 N, 79 58 56 W', 40.4461111, -79.9822222],
  ]) {
    const read = await readCoordinate(text, ctx);
    assert.ok(read.ok, `${text}: ${read.message}`);
    assert.equal(degrees(read.lat), String(lat), text);
    assert.equal(degrees(read.lon), String(lon), text);
  }
});

test('text that is not a coordinate says so instead of guessing', async () => {
  const read = await readCoordinate('the back paddock', ctx);
  assert.equal(read.ok, false);
  assert.match(read.message, /not a coordinate/);
  assert.deepEqual(await readCoordinate('   ', ctx), { ok: false, message: '' });
});

test('only a tool with one latitude and one longitude gets the field', () => {
  assert.deepEqual(pairOf(byId('indexing.geohash.encode')), { lat: 'lat', lon: 'lon' });
  // Two points is two gestures, not one field.
  assert.equal(pairOf(byId('navigation.geodesic.inverse')), null);
  assert.equal(pairOf(byId('aviation.altimetry.density-altitude')), null);
});

test('the field is on the pages that have a pair, and nowhere else', () => {
  const page = (id) => readFileSync(join(web, 'dist', ...id.split('.'), 'index.html'), 'utf8');
  const problems = [];
  for (const t of catalog.tools) {
    const has = /class="coordinate-field"/.test(page(t.id));
    const wants = pairOf(t) !== null;
    if (has !== wants) problems.push(`${t.id}: ${has ? 'has' : 'lacks'} the field, ${wants ? 'wants' : 'does not want'} it`);
  }
  assert.deepEqual(problems, []);
  assert.ok(catalog.tools.filter((t) => pairOf(t)).length >= 20, 'too few tools to be a real sweep');
});

test('degrees are written the way the field takes them', () => {
  assert.equal(degrees(40.446111111111), '40.4461111');
  assert.equal(degrees(-0), '0');
  assert.equal(degrees(12), '12');
});
