// Content minimums for indexable pages (discovery/search-pages). A page that
// search engines should rank has to say something of its own: its purpose, its
// own worked example laid out as "You enter / You get", the answer in the HTML
// before any script, and when its sources were last checked.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const dist = join(web, 'dist');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const route = (id) => '/' + id.split('.').join('/') + '/';
const page = (r) => readFileSync(join(dist, r, 'index.html'), 'utf8');
const indexable = catalog.tools.filter((t) => t.stability === 'stable' && t.composedOf.length === 0);

test('every indexable page states a purpose no other page states', () => {
  const seen = new Map();
  const problems = [];
  for (const t of indexable) {
    const html = page(route(t.id));
    assert.match(html, /<h1[ >]/, `${t.id} has no H1`);
    if (!html.includes(t.summary)) problems.push(`${t.id}: its purpose is not in the HTML`);
    const first = seen.get(t.summary);
    if (first) problems.push(`${t.id} and ${first} share a purpose statement`);
    else seen.set(t.summary, t.id);
  }
  assert.deepEqual(problems, []);
});

test('no two indexable pages show the same worked example', () => {
  // Related tools legitimately start from the same coordinate; what may not
  // repeat is the whole example, inputs and answer together, which is what a
  // crawler would see as the same page.
  const seen = new Map();
  const problems = [];
  for (const t of indexable) {
    const block = /<div class="worked(?: no-print)?">([\s\S]*?)<\/div>\s*<\/div>/.exec(page(route(t.id)))?.[1] ?? '';
    const key = block.replace(/<[^>]+>/g, '|').replace(/\s+/g, ' ');
    const first = seen.get(key);
    if (first) problems.push(`${t.id} and ${first} show the same worked example`);
    else seen.set(key, t.id);
  }
  assert.deepEqual(problems, []);
});

test('every indexable page lays the example out as "You enter / You get"', () => {
  const problems = [];
  for (const t of indexable) {
    const html = page(route(t.id));
    const block = /<div class="worked(?: no-print)?">([\s\S]*?)<\/div>\s*<\/div>/.exec(html)?.[1];
    if (!block) {
      problems.push(`${t.id}: no worked-example block`);
      continue;
    }
    if (!/<h3>You enter<\/h3>/.test(block)) problems.push(`${t.id}: no "You enter"`);
    if (!/<h3>You get<\/h3>/.test(block)) problems.push(`${t.id}: no "You get"`);
    const rows = [...block.matchAll(/<dd>/g)].length;
    if (rows < 2) problems.push(`${t.id}: the worked example has ${rows} rows`);
  }
  assert.deepEqual(problems, []);
});

test('the answer is in the HTML, ahead of the island that would recompute it', () => {
  const problems = [];
  for (const t of catalog.tools) {
    const html = page(route(t.id));
    const answer = html.indexOf('<div class="value"');
    const island = html.indexOf('<astro-island');
    if (answer < 0) problems.push(`${t.id}: no answer in the HTML`);
    else if (island >= 0 && island > answer) problems.push(`${t.id}: the island is declared after the answer it hydrates`);
    // With scripts off, the answer still reads: it is plain text in the markup,
    // a number for most tools and a word for the decoders ("direct", "KDEN").
    else if (!/<div class="value"[^>]*>\s*[^\s<]/.test(html)) problems.push(`${t.id}: the answer is not plain text`);
  }
  assert.deepEqual(problems, []);
});

test('every indexable page says when its sources were last checked', () => {
  const problems = [];
  for (const t of indexable) {
    if (!/<strong>Last verified:<\/strong> \d{4}-\d{2}-\d{2}/.test(page(route(t.id)))) {
      problems.push(`${t.id}: no last-verified date`);
    }
  }
  assert.deepEqual(problems, []);
});

/** The prose a page says itself, counting only manifest fields, never template text. */
const ownWords = (t) =>
  [t.summary, t.whenToUse, t.limitations, t.accuracy, ...(t.related ?? []).map((r) => r.reason), ...(t.examples ?? []).map((e) => `${e.title} ${e.source}`)]
    .join(' ')
    .trim()
    .split(/\s+/)
    .filter(Boolean).length;

test('every indexable page carries 150 words of its own prose', () => {
  // The floor counts the manifest's own fields only: a page that would meet it
  // on shared template text is exactly the thin page this is meant to catch.
  const thin = indexable.filter((t) => ownWords(t) < 150).map((t) => `${t.id}: ${ownWords(t)} words`);
  assert.deepEqual(thin, []);
});

test('when-to-use and limitations are written per tool, not shared', () => {
  const problems = [];
  for (const field of ['whenToUse', 'limitations']) {
    const seen = new Map();
    for (const t of indexable) {
      const v = t[field];
      if (!v) {
        problems.push(`${t.id} has no ${field}`);
        continue;
      }
      if (v.split(/\s+/).length < 35) problems.push(`${t.id}: ${field} is ${v.split(/\s+/).length} words`);
      const first = seen.get(v);
      if (first) problems.push(`${t.id} and ${first} share their ${field}`);
      else seen.set(v, t.id);
    }
  }
  assert.deepEqual(problems, []);
  // And the page shows them, so the words are on the page and not only in the
  // manifest. Astro escapes the text, so the check escapes it the same way.
  const esc = (t) => t.replace(/&/g, '&#38;').replace(/'/g, '&#39;').replace(/"/g, '&#34;').replace(/</g, '&#60;').replace(/>/g, '&#62;');
  for (const t of indexable) {
    const html = page(route(t.id));
    assert.ok(html.includes(esc(t.whenToUse)), `${t.id}: when-to-use is not rendered`);
    assert.ok(html.includes(esc(t.limitations)), `${t.id}: limitations are not rendered`);
  }
});
