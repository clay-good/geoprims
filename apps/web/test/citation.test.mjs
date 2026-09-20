// Citations that travel with the result (trust/citations). One renderer writes
// a citation for the page, the printed sheet, and the copied answer, and the
// core puts the same short form in every result's meta.references.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { freeAccess, isPaid, shortCitation, withReference } from '../src/lib/citation.mjs';
import { copyText, FORMATS } from '../src/lib/copy.js';
import { safeLinks, unsafeLinks } from '../scripts/links.mjs';
import { nodeHost } from '../../../packages/runtime/src/node.mjs';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const ledger = JSON.parse(readFileSync(join(root, 'data/sources-ledger.json'), 'utf8')).sources;
const page = (id) => readFileSync(join(web, 'dist', ...id.split('.'), 'index.html'), 'utf8');
const host = nodeHost(join(root, 'dist/wasm'));

test('every result carries the sources it came from', async () => {
  const t = catalog.tools.find((x) => x.id === 'survey.reduction.combined-factor');
  const args = (t.examples.find((e) => e.id === t['x-primary-example']) ?? t.examples[0]).input;
  const out = JSON.parse(await host.invoke(t.id, JSON.stringify(args)));
  assert.ok(out.ok, JSON.stringify(out).slice(0, 200));
  assert.ok(out.meta.references?.length, 'no references in meta');
  for (const r of out.meta.references) {
    assert.deepEqual(Object.keys(r).sort(), ['edition', 'issuer', 'locator', 'title']);
    for (const k of Object.keys(r)) assert.ok(r[k], `${k} is empty`);
  }
});

test('copy with reference carries everything needed to defend the number', async () => {
  // The scenario: a surveyor copies a combined-factor result with reference.
  const t = catalog.tools.find((x) => x.id === 'survey.reduction.combined-factor');
  const args = (t.examples.find((e) => e.id === t['x-primary-example']) ?? t.examples[0]).input;
  const result = JSON.parse(await host.invoke(t.id, JSON.stringify(args)));
  const text = withReference({
    answer: '0.99988',
    result,
    tool: { ...t, coreVersion: result.meta.coreVersion },
    args,
    href: 'https://geoprims.com/survey/reduction/combined-factor/#v1:abc',
    today: '2026-09-20',
  });
  assert.match(text, /0\.99988/, 'no answer');
  assert.match(text, new RegExp(Object.keys(args)[0].replace(/_/g, '.'), 'i'));
  assert.match(text, /Method: /, 'no method');
  assert.match(text, /Sources:/, 'no citations');
  for (const r of result.meta.references) assert.ok(text.includes(r.locator), `no locator for ${r.title}`);
  assert.match(text, new RegExp(`geoprims ${t.id.replace(/\./g, '\\.')} ${t.version.replace(/\./g, '\\.')}`));
  assert.match(text, /core \d/, 'no core version');
  assert.match(text, /copied 2026-09-20/, 'no date');
  assert.match(text, /not a legal survey determination/i, 'no notice');
  assert.equal(copyText('reference', { answer: '0.99988', result, tool: { ...t, coreVersion: result.meta.coreVersion }, args, href: '', today: '2026-09-20' }), text.replace('\nhttps://geoprims.com/survey/reduction/combined-factor/#v1:abc', ''));
  assert.ok(FORMATS.includes('reference'));
});

test('the page offers the copy the scenario describes', () => {
  assert.match(page('survey.reduction.combined-factor'), />Copy with reference</);
});

test('a citation reads the same everywhere', () => {
  const ref = { issuer: 'NOAA NCEI', title: 'World Magnetic Model 2025', edition: 'WMM2025', locator: 'Section 3', url: 'https://www.ncei.noaa.gov/products/world-magnetic-model' };
  assert.equal(shortCitation(ref), 'NOAA NCEI, World Magnetic Model 2025, WMM2025. Section 3.');
});

test('a document that is sold is never called free', () => {
  const icao = { url: 'https://store.icao.int/en/doc-7488' };
  assert.ok(isPaid(icao.url));
  assert.deepEqual(freeAccess(icao, { freeAccessUrl: icao.url }), { url: null, label: 'Paid only' });
  assert.deepEqual(freeAccess(icao, { freeAccessUrl: 'https://store.icao.int/en/other' }), { url: 'https://store.icao.int/en/other', label: 'Paid only' });
  const free = { url: 'https://example.org/paper.pdf' };
  assert.deepEqual(freeAccess(free, { freeAccessUrl: 'https://ntrs.nasa.gov/citations/1' }), { url: 'https://ntrs.nasa.gov/citations/1', label: 'Read free' });
  assert.equal(freeAccess(free, { freeAccessUrl: free.url }), null);
});

test('every cited link points at a document, not a home page', () => {
  const problems = [];
  const deep = (url) => {
    try {
      const u = new URL(url);
      return u.pathname.replace(/\/+$/, '').length > 1 || u.search.length > 1;
    } catch {
      return false;
    }
  };
  for (const row of ledger) {
    if (!deep(row.freeAccessUrl)) problems.push(`${row.id}: ${row.freeAccessUrl} is a home page`);
  }
  for (const t of catalog.tools) {
    for (const r of t.references) if (!deep(r.url)) problems.push(`${t.id}: ${r.url} is a home page`);
  }
  assert.deepEqual([...new Set(problems)], []);
});

test('no link leaves the site without noopener and noreferrer', () => {
  for (const id of ['aviation.altimetry.density-altitude', 'survey.reduction.combined-factor']) {
    assert.deepEqual(unsafeLinks(page(id)), []);
  }
  // And the pass that puts them there merges rather than replaces.
  const { html } = safeLinks('<a href="https://x.test/doc" rel="me">x</a><a href="/local">y</a>');
  assert.match(html, /rel="me noopener noreferrer"/);
  assert.match(html, /<a href="\/local">/);
  assert.deepEqual(unsafeLinks(html), []);
});

test('the gate bites', () => {
  assert.deepEqual(unsafeLinks('<a href="https://x.test/a">x</a>'), ['<a href="https://x.test/a">']);
  assert.deepEqual(unsafeLinks('<a href="https://x.test/a" rel="noopener">x</a>'), ['<a href="https://x.test/a" rel="noopener">']);
});
