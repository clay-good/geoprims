// The page-versus-endpoint rule (discovery/search-pages). A generated
// conversion endpoint is a tool id and a search alias; it becomes a page of
// its own only with recorded demand or a dated owner decision behind it,
// because a site full of near-identical conversion pages is exactly what
// scaled-content policies are aimed at.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const root = new URL('../..', import.meta.url).pathname;
const list = JSON.parse(readFileSync(join(root, 'data/seo/high-intent-pages.json'), 'utf8'));
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const page = (id) => readFileSync(join(root, 'apps/web/dist', ...id.split('.'), 'index.html'), 'utf8');
const endpoints = catalog.tools.filter((t) => t.composedOf.length > 0);
const listed = new Set(list.pages.map((p) => p.id));

/** Everything wrong with the list itself. */
export function listProblems({ pages, maxPages }, ids) {
  const problems = [];
  if (pages.length > maxPages) problems.push(`${pages.length} pages listed (at most ${maxPages})`);
  const seen = new Set();
  for (const p of pages) {
    if (!ids.has(p.id)) problems.push(`${p.id} is not a generated endpoint`);
    if (seen.has(p.id)) problems.push(`${p.id} is listed twice`);
    seen.add(p.id);
    if (!p.justification) problems.push(`${p.id}: no justification`);
    const date = (d) => /^\d{4}-\d{2}-\d{2}$/.test(d ?? '');
    if (p.decision !== undefined) {
      // An owner decision stands in for a measurement only as itself: dated,
      // and due to be checked against Search Console.
      if (p.decision !== 'owner') problems.push(`${p.id}: a decision is made by the owner`);
      if (p.source || p.measured) problems.push(`${p.id}: a decision is not a measurement; give one or the other`);
      if (!date(p.decided)) problems.push(`${p.id}: no date the decision was made`);
      if (!date(p.reviewBy) || !(p.reviewBy > p.decided)) problems.push(`${p.id}: no review date after the decision`);
    } else {
      if (!p.source) problems.push(`${p.id}: no source for the demand`);
      if (!date(p.measured)) problems.push(`${p.id}: no date the demand was measured`);
    }
  }
  return problems;
}

test('the list is well formed and within its cap', () => {
  assert.ok(endpoints.length > 0, 'no generated endpoints to rule on');
  assert.deepEqual(listProblems(list, new Set(endpoints.map((t) => t.id))), []);
});

test('a listed endpoint is a page of its own: self-canonical, indexable, and in its sitemap', () => {
  const dir = join(root, 'apps/web/dist/sitemaps');
  const all = readdirSync(dir).map((f) => readFileSync(join(dir, f), 'utf8')).join('');
  const problems = [];
  for (const id of listed) {
    const url = `https://geoprims.com/${id.split('.').join('/')}/`;
    const html = page(id);
    const canonical = /<link rel="canonical" href="([^"]+)"/.exec(html)?.[1];
    if (canonical !== url) problems.push(`${id}: canonical ${canonical}, expected ${url}`);
    if (/<meta name="robots" content="noindex">/.test(html)) problems.push(`${id}: noindex`);
    if (!all.includes(`<loc>${url}</loc>`)) problems.push(`${id}: not in a sitemap`);
  }
  assert.deepEqual(problems, []);
  assert.match(list.justificationRequired, /Search Console/);
});

test('every endpoint not on the list renders as a preset of its parent', () => {
  const problems = [];
  for (const t of endpoints) {
    if (listed.has(t.id)) continue;
    const html = page(t.id);
    const canonical = /<link rel="canonical" href="([^"]+)"/.exec(html)?.[1];
    const parent = t.composedOf[0];
    const parentUrl = `https://geoprims.com/${parent.split('.').join('/')}/`;
    if (canonical !== parentUrl) problems.push(`${t.id}: canonical ${canonical}, expected ${parentUrl}`);
    if (!/<meta name="robots" content="noindex">/.test(html)) problems.push(`${t.id}: not noindex`);
    // The preset is applied rather than asked for: the field it fixes is gone
    // from the form, nothing in the example contradicts it, and the page still
    // ships an answer. What that answer reads is checked by the example gates.
    const answer = (/<div class="value"[^>]*>([\s\S]*?)<\/div>/.exec(html)?.[1] ?? '').replace(/<[^>]+>/g, ' ').trim();
    if (!answer) problems.push(`${t.id}: no answer on the page`);
    for (const [field, value] of Object.entries(t.preset ?? {})) {
      if (new RegExp(`id="field-${field}"`).test(html)) problems.push(`${t.id}: ${field} is still a field to fill in`);
      if (field in t.inputs.properties) problems.push(`${t.id}: ${field} is still an input of the endpoint`);
      const given = t.examples[0]?.input?.[field];
      if (given !== undefined && String(given) !== String(value)) {
        problems.push(`${t.id}: the example sets ${field}=${given}, but the endpoint presets ${value}`);
      }
    }
  }
  assert.deepEqual(problems, []);
});

test('no endpoint page is in a sitemap', () => {
  const dir = join(root, 'apps/web/dist/sitemaps');
  const all = readdirSync(dir).map((f) => readFileSync(join(dir, f), 'utf8')).join('');
  const problems = endpoints
    .filter((t) => !listed.has(t.id))
    .filter((t) => all.includes(`https://geoprims.com/${t.id.split('.').join('/')}/`))
    .map((t) => `${t.id} is in a sitemap`);
  assert.deepEqual(problems, []);
});

test('the rule bites', () => {
  const ids = new Set(['units.length.ft-to-m']);
  assert.deepEqual(listProblems({ maxPages: 60, pages: [{ id: 'units.length.made-up', justification: 'x', source: 'y', measured: '2026-09-20' }] }, ids), [
    'units.length.made-up is not a generated endpoint',
  ]);
  assert.deepEqual(listProblems({ maxPages: 60, pages: [{ id: 'units.length.ft-to-m' }] }, ids), [
    'units.length.ft-to-m: no justification',
    'units.length.ft-to-m: no source for the demand',
    'units.length.ft-to-m: no date the demand was measured',
  ]);
  const decided = { id: 'units.length.ft-to-m', justification: 'a', decision: 'owner', decided: '2026-10-07', reviewBy: '2027-01-05' };
  assert.deepEqual(listProblems({ maxPages: 60, pages: [decided] }, ids), []);
  assert.deepEqual(listProblems({ maxPages: 60, pages: [{ ...decided, decision: 'agent', measured: '2026-10-07', reviewBy: '2026-10-01' }] }, ids), [
    'units.length.ft-to-m: a decision is made by the owner',
    'units.length.ft-to-m: a decision is not a measurement; give one or the other',
    'units.length.ft-to-m: no review date after the decision',
  ]);
  assert.deepEqual(listProblems({ maxPages: 1, pages: [{ id: 'units.length.ft-to-m', justification: 'a', source: 'b', measured: '2026-09-20' }, { id: 'units.length.ft-to-m', justification: 'a', source: 'b', measured: '2026-09-20' }] }, ids), [
    '2 pages listed (at most 1)',
    'units.length.ft-to-m is listed twice',
  ]);
});
