// Glossary gate: the committed glossary resolves every abbreviation in the
// built catalog, and the gate catches a missing term (the "RPP" scenario),
// an overlong definition, and an unknown source.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { glossaryProblems, loadGlossary } from './glossary.mjs';

const root = new URL('../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const ledger = JSON.parse(readFileSync(join(root, 'data/sources-ledger.json'), 'utf8'));
const ledgerIds = new Set(Object.values(ledger).find(Array.isArray).map((r) => r.id));

test('every abbreviation in the catalog resolves to a glossary entry', () => {
  assert.deepEqual(glossaryProblems(loadGlossary(root), catalog, ledgerIds), []);
});

test('a label using an undefined abbreviation fails, naming the tool and term', () => {
  const tool = { id: 'fixture.qzx', title: 'Weight of the QZX kit', inputs: { properties: {} }, outputs: { properties: {} } };
  const problems = glossaryProblems(loadGlossary(root), { tools: [...catalog.tools, tool] }, ledgerIds);
  assert.deepEqual(problems, ['fixture.qzx: "QZX" has no glossary entry']);
});

test('the schema catches an overlong definition and an unknown source', () => {
  const g = loadGlossary(root);
  g.entries[0] = { ...g.entries[0], definition: 'word '.repeat(41).trim(), source: 'no-such-row' };
  const problems = glossaryProblems(g, catalog, ledgerIds);
  assert.ok(problems.includes(`${g.entries[0].term}: the definition has 41 words (at most 40)`), problems.join('\n'));
  assert.ok(problems.includes(`${g.entries[0].term}: source "no-such-row" is not a sources-ledger row`), problems.join('\n'));
});
