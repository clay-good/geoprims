// No copyrighted table is reproduced in a flagged tool, and each one takes the
// value a reader would have looked up as an input instead.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { FLAGGED, TABLE_RUN, tableProblems, tableRuns } from './tables.mjs';

const root = join(new URL('.', import.meta.url).pathname, '../..');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));

test('no flagged tool has a table typed into it', () => {
  assert.deepEqual(tableProblems(root, catalog), []);
});

test('the sight-distance value is an input citing the table to look it up in', () => {
  // The scenario: the design K value is an input, and the Green Book table is
  // cited by edition and number, with no table reproduced.
  const t = catalog.tools.find((x) => x.id === 'survey.curves.vertical-curve');
  const k = t.inputs.properties.k;
  assert.ok(k, 'K is not an input');
  assert.match(k['x-help'], /AASHTO Green Book Table 3-34/);
  const ref = t.references.find((r) => /Green Book/.test(r.title));
  assert.ok(ref, 'the Green Book is not cited');
  assert.equal(ref.edition, '7th edition');
  assert.match(ref.locator, /Table 3-34/);
  // Nothing from the table itself: no speed-to-K pairs anywhere in the tool.
  const src = readFileSync(join(root, 'core/crates/gp-survey/src/lib.rs'), 'utf8');
  assert.ok(!/design_k|K_BY_SPEED|k_table/i.test(src), 'a design-K table is in the source');
});

test('the lint sees a table when one is there', () => {
  const copied = `
    id: "survey.curves.vertical-curve",
    const K_BY_SPEED: [f64; 9] = [2.0, 4.0, 7.0, 12.0, 19.0, 29.0, 44.0, 61.0, 84.0];
  `;
  const runs = tableRuns(copied);
  assert.equal(runs.length, 1);
  assert.equal(runs[0].count, 9);
  assert.ok(runs[0].count >= TABLE_RUN);
});

test('the lint leaves an index or a shape alone', () => {
  assert.deepEqual(tableRuns('let dims = [1, 2, 3, 4, 5, 6, 7, 8, 9];'), []);
  assert.deepEqual(tableRuns('let empty: [f64; 0] = [];'), []);
  // A list of fields or examples holds numbers, but it is not a table.
  const fields = 'inputs: &[Field::new("g1", "g1", "Percent, like +2", Kind::Number { min: -100.0, max: 100.0 }), Field::new("g2", "g2", "like -3", Kind::Number { min: -100.0, max: 100.0 })],';
  assert.deepEqual(tableRuns(fields), []);
});

test('the lint reads the tool, not its tests', () => {
  const src = `id: "survey.curves.vertical-curve",\n#[cfg(test)]\nmod tests { const T: [f64; 9] = [2.0, 4.0, 7.0, 12.0, 19.0, 29.0, 44.0, 61.0, 84.0]; }`;
  assert.deepEqual(tableRuns(src), []);
});

test('every flagged tool is real, and says what governs it', () => {
  for (const f of FLAGGED) {
    assert.ok(catalog.tools.some((t) => t.id === f.id), `${f.id} is not a tool`);
    assert.ok(f.governs.length > 20, `${f.id}: no statement of what governs it`);
    assert.ok(readFileSync(join(root, f.source), 'utf8').includes(`id: "${f.id}"`), `${f.id} is not in ${f.source}`);
  }
});
