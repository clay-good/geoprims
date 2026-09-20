// Plain vocabulary and glossary (ux/glanceable-results). An abbreviation a
// page uses has to explain itself on tap, in at most 40 words, citing where
// the definition comes from — and every input has a one-line help text with an
// example value in it.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { defId, definitionProblems, markTerms, MAX_WORDS } from '../src/lib/terms.mjs';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const glossary = JSON.parse(readFileSync(join(root, 'data/glossary.json'), 'utf8'));
const ledger = Object.values(JSON.parse(readFileSync(join(root, 'data/sources-ledger.json'), 'utf8'))).find(Array.isArray);
const sourceById = new Map(ledger.map((r) => [r.id, r]));
const page = (id) => readFileSync(join(web, 'dist', ...id.split('.'), 'index.html'), 'utf8');

test('every definition is short enough to read and cites its source', () => {
  const problems = [];
  for (const e of glossary.entries) problems.push(...definitionProblems(e, sourceById));
  assert.deepEqual(problems, []);
});

test('only the first use of a term is a button', () => {
  const entries = [{ id: 'hae', term: 'HAE', expansion: 'height above ellipsoid', definition: 'd', source: 'egm96' }];
  const { html, used } = markTerms('HAE, then HAE again, and HAEX is something else.', entries);
  assert.equal((html.match(/class="term"/g) ?? []).length, 1);
  assert.match(html, /^<button[^>]*>HAE<\/button>, then HAE again, and HAEX/);
  assert.deepEqual(used, entries);
});

test('a term that does not appear gets no button and no popover', () => {
  const entries = [{ id: 'msl', term: 'MSL', expansion: 'mean sea level', definition: 'd', source: 'egm96' }];
  const { html, used } = markTerms('Nothing to mark here.', entries);
  assert.equal(html, 'Nothing to mark here.');
  assert.deepEqual(used, []);
});

test('the text a page shows is escaped before any markup goes in', () => {
  const { html } = markTerms('<script>alert(1)</script> & HAE', [{ id: 'hae', term: 'HAE', expansion: 'e', definition: 'd', source: 'egm96' }]);
  assert.ok(!html.includes('<script>'), html);
  assert.match(html, /&amp;/);
});

test('every button on a page has the popover it opens, and every popover a button', () => {
  const problems = [];
  let pages = 0;
  for (const t of catalog.tools) {
    const html = page(t.id);
    const buttons = [...html.matchAll(/popovertarget="([^"]+)"/g)].map((m) => m[1]);
    const popovers = [...html.matchAll(/<div popover id="([^"]+)"/g)].map((m) => m[1]);
    if (buttons.length) pages += 1;
    for (const id of buttons) if (!popovers.includes(id)) problems.push(`${t.id}: ${id} opens nothing`);
    for (const id of popovers) if (!buttons.includes(id)) problems.push(`${t.id}: ${id} is opened by nothing`);
    assert.equal(new Set(popovers).size, popovers.length, `${t.id}: a popover id is used twice`);
  }
  assert.deepEqual(problems, []);
  assert.ok(pages >= 40, `only ${pages} pages define a term on tap`);
});

test('the HAE definition is the one the scenario asks for', () => {
  const hae = glossary.entries.find((e) => e.term === 'HAE');
  assert.ok(hae, 'no HAE entry');
  assert.equal(defId(hae), 'def-hae');
  assert.match(hae.expansion, /height above ellipsoid/i);
  assert.match(hae.definition, /ellipsoid/i);
  assert.ok(hae.definition.split(/\s+/).length <= MAX_WORDS);
  assert.ok(sourceById.has(hae.source), `HAE cites unknown source ${hae.source}`);
});

/** Every input a reader fills, including the fields inside a list's rows. */
function* inputFields(t) {
  for (const [name, schema] of Object.entries(t.inputs.properties)) {
    if (name === 'options') continue;
    yield [name, schema];
    for (const [row, rowSchema] of Object.entries(schema.items?.properties ?? {})) {
      yield [`${name}[].${row}`, rowSchema];
    }
  }
}

test('every input has one line of help with an example value in it', () => {
  const problems = [];
  let checked = 0;
  for (const t of catalog.tools) {
    for (const [name, schema] of inputFields(t)) {
      checked += 1;
      const help = schema['x-help'] ?? '';
      if (!help) problems.push(`${t.id}: ${name} has no help`);
      else if (help.includes('\n')) problems.push(`${t.id}: ${name} help is more than one line`);
      // A concrete value: a number, a coded example, or the choices themselves.
      else if (!/\d/.test(help) && !schema.enum && !/like |e\.g\./i.test(help)) {
        problems.push(`${t.id}: ${name} help shows no example value: "${help}"`);
      }
    }
  }
  assert.deepEqual(problems, []);
  assert.ok(checked > 600, `only ${checked} inputs scanned`);
});
