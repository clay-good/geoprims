// The built catalog uses only the contract's manifest extensions, and the
// gate catches the spec's scenarios: an unknown x-color and six core inputs.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { coreGroup, metaschemaProblems } from './metaschema.mjs';

const root = new URL('../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const ledger = JSON.parse(readFileSync(join(root, 'data/sources-ledger.json'), 'utf8'));
const sourceIds = new Set(Object.values(ledger).find(Array.isArray).map((r) => r.id));

test('the catalog carries only contract extensions, well formed', () => {
  assert.deepEqual(metaschemaProblems(catalog, sourceIds), []);
});

test('an unknown extension fails, naming the field', () => {
  const t = structuredClone(catalog.tools[0]);
  t['x-color'] = 'orange';
  const name = Object.keys(t.inputs.properties)[0];
  t.inputs.properties[name]['x-shade'] = 'dark';
  assert.deepEqual(metaschemaProblems({ tools: [t] }), [
    `${t.id}: unknown extension x-color`,
    `${t.id}: unknown extension x-shade on inputs.${name}`,
  ]);
});

test('six core inputs fail; a list of rows counts once', () => {
  const field = { type: 'number', 'x-core': true };
  const t = { id: 'fixture.core', examples: [], inputs: { properties: Object.fromEntries([1, 2, 3, 4, 5, 6].map((i) => [`f${i}`, field])) }, outputs: { properties: {} } };
  assert.deepEqual(metaschemaProblems({ tools: [t] }), ['fixture.core: 6 x-core inputs (at most 5)']);
  const rows = { type: 'array', 'x-core': true, items: { properties: { weight: field, arm: field } } };
  const w = { id: 'fixture.rows', examples: [], inputs: { properties: { a: field, b: field, c: field, d: field, stations: rows } }, outputs: { properties: {} } };
  assert.deepEqual(metaschemaProblems({ tools: [w] }), []);
});

test('a status output must name a kind, a real source, and be text', () => {
  const status = { type: 'string', 'x-status': { kind: 'threshold', source: 'user' } };
  const ok = { id: 'fixture.status', examples: [], inputs: { properties: {} }, outputs: { properties: { s: status } } };
  assert.deepEqual(metaschemaProblems({ tools: [ok] }, sourceIds), []);

  const bad = structuredClone(ok);
  bad.outputs.properties.s['x-status'].kind = 'vibes';
  bad.outputs.properties.s['x-status'].source = 'no-such-source';
  bad.outputs.properties.s.type = 'number';
  assert.deepEqual(metaschemaProblems({ tools: [bad] }, sourceIds), [
    'fixture.status: x-status kind vibes on outputs.s is not one of threshold, conformance',
    'fixture.status: x-status on outputs.s cites unknown source no-such-source',
    'fixture.status: x-status output outputs.s must be a string',
  ]);

  const onInput = { id: 'fixture.in', examples: [], inputs: { properties: { s: structuredClone(status) } }, outputs: { properties: {} } };
  assert.deepEqual(metaschemaProblems({ tools: [onInput] }, sourceIds), ['fixture.in: x-status is for outputs, not inputs.s']);
});

test('a comparison must declare its kind and carry the line it renders', () => {
  const base = { id: 'fixture.c', examples: [], inputs: { properties: {} }, outputs: { properties: {} } };
  assert.deepEqual(metaschemaProblems({ tools: [{ ...base, 'x-comparison': { kind: 'none' } }] }, sourceIds), []);
  assert.deepEqual(metaschemaProblems({ tools: [{ ...base, 'x-comparison': { kind: 'vs-rule-of-thumb', text: 'The rule gives {x}.' } }] }, sourceIds), []);
  assert.deepEqual(metaschemaProblems({ tools: [{ ...base, 'x-comparison': { kind: 'vs-rule-of-thumb' } }] }, sourceIds), [
    'fixture.c: x-comparison kind vs-rule-of-thumb needs text',
  ]);
  assert.deepEqual(metaschemaProblems({ tools: [{ ...base, 'x-comparison': { kind: 'none', text: 'anything' } }] }, sourceIds), [
    'fixture.c: x-comparison kind none takes no text',
  ]);
  assert.deepEqual(metaschemaProblems({ tools: [{ ...base, 'x-comparison': { kind: 'vibes', text: 'x' } }] }, sourceIds), [
    'fixture.c: x-comparison kind vibes is not one of vs-input, vs-rule-of-thumb, vs-typical-range, none',
  ]);
});

test('a limitation banner must carry all three lines, within their caps', () => {
  const base = { id: 'fixture.l', examples: [], inputs: { properties: {} }, outputs: { properties: {} } };
  const good = { simplification: 'Assumes dry air.', instead: 'Add a dew point.', governs: 'ICAO Doc 7488.' };
  assert.deepEqual(metaschemaProblems({ tools: [{ ...base, 'x-limitation': good }] }, sourceIds), []);
  assert.deepEqual(metaschemaProblems({ tools: [{ ...base, 'x-limitation': { ...good, instead: undefined } }] }, sourceIds), [
    'fixture.l: x-limitation needs instead',
  ]);
  assert.deepEqual(metaschemaProblems({ tools: [{ ...base, 'x-limitation': { ...good, simplification: 'x'.repeat(81) } }] }, sourceIds), [
    'fixture.l: x-limitation simplification is 81 characters (at most 80)',
  ]);
  assert.deepEqual(metaschemaProblems({ tools: [{ ...base, 'x-limitation': { ...good, colour: 'orange' } }] }, sourceIds), [
    'fixture.l: x-limitation has no field colour',
  ]);
});

test('a coordinate counts as one core input, the way a list of rows does', () => {
  assert.equal(coreGroup('lat1'), coreGroup('lon1'), 'lat1 and lon1 are one point');
  assert.notEqual(coreGroup('lat1'), coreGroup('lat2'), 'two points are two things');
  assert.equal(coreGroup('b_north'), coreGroup('b_east'), 'a named easting and northing are one point');
  assert.equal(coreGroup('temperature'), 'temperature', 'anything else counts as itself');
  const core = { type: 'number', 'x-core': true };
  const props = Object.fromEntries(['lat1', 'lon1', 'lat2', 'lon2', 'lat', 'lon', 'ellipsoid'].map((k) => [k, core]));
  const t = { id: 'fixture.points', examples: [], inputs: { properties: props }, outputs: { properties: {} } };
  assert.deepEqual(metaschemaProblems({ tools: [t] }, sourceIds), [], 'three points and a choice is four inputs');
  // Six genuinely separate inputs still fail.
  const six = Object.fromEntries(['a', 'b', 'c', 'd', 'e', 'f'].map((k) => [k, core]));
  assert.deepEqual(metaschemaProblems({ tools: [{ ...t, id: 'fixture.six', inputs: { properties: six } }] }, sourceIds), [
    'fixture.six: 6 x-core inputs (at most 5)',
  ]);
});

test('a step pair that is not a pair of positive, ordered numbers is rejected', () => {
  const t = (step) => ({ id: 'fixture.step', examples: [], inputs: { properties: { v: { type: 'number', 'x-step': step } } }, outputs: { properties: {} } });
  assert.deepEqual(metaschemaProblems({ tools: [t({ small: 0.01, large: 0.1 })] }), []);
  assert.deepEqual(metaschemaProblems({ tools: [t({ small: 1, large: 1 })] }), [
    'fixture.step: x-step on inputs.v has a small step of 1 that is not smaller than 1',
  ]);
  assert.deepEqual(metaschemaProblems({ tools: [t({ small: 0, large: 10 })] }), [
    'fixture.step: x-step on inputs.v needs positive small and large steps',
  ]);
  assert.deepEqual(metaschemaProblems({ tools: [t({ small: 1, large: 10, medium: 5 })] }), [
    'fixture.step: x-step has no field medium',
  ]);
});
