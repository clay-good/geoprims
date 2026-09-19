// The built catalog uses only the contract's manifest extensions, and the
// gate catches the spec's scenarios: an unknown x-color and six core inputs.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { metaschemaProblems } from './metaschema.mjs';

const root = new URL('../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));

test('the catalog carries only contract extensions, well formed', () => {
  assert.deepEqual(metaschemaProblems(catalog), []);
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
