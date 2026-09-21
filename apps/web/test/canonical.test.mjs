// Canonical URLs and state (discovery/search-pages, "Canonical URLs and
// state"). A tool page is one URL however many values a reader has typed into
// it: the values live in the fragment, which never reaches a server or a
// crawler, and the canonical is always the clean path.
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
const canonicalOf = (html) => /<link rel="canonical" href="([^"]+)"/.exec(html)?.[1];

test('a permalink and the page it came from share one canonical', () => {
  // A crawler fetching https://geoprims.com/…/#v1:… asks for the path only:
  // the fragment never leaves the browser, so the canonical it reads is the
  // clean path the page declares.
  const problems = [];
  for (const t of catalog.tools) {
    const html = page(route(t.id));
    const canonical = canonicalOf(html);
    if (!canonical) problems.push(`${t.id}: no canonical`);
    else if (canonical.includes('#')) problems.push(`${t.id}: the canonical carries a fragment`);
    else if (canonical.includes('?')) problems.push(`${t.id}: the canonical carries a query string`);
  }
  assert.deepEqual(problems, []);
});

test('the values a reader types stay in the fragment', () => {
  // The permalink the page writes is a fragment, and the only query string
  // anywhere is the catalog filter, which is a search, not a tool input.
  const app = readFileSync(join(web, 'src/components/ToolApp.svelte'), 'utf8');
  // The core encodes the link; the page writes it as a fragment, never a path
  // or a query.
  assert.match(app, /history\.replaceState\(null, '', `#\$\{enc\.result\.fragment\}`\)/, 'the permalink is not written as a fragment');
  assert.ok(!/history\.(replaceState|pushState)\([^)]*`\?/.test(app), 'a permalink goes into a query string');
  assert.ok(!/searchParams\.set\('(?!q\b)/.test(app), 'a tool input goes into a query string');
  const problems = [];
  for (const t of catalog.tools.slice(0, 40)) {
    for (const [, href] of page(route(t.id)).matchAll(/href="(\/[^"]*\?[^"]*)"/g)) {
      if (!/^\/tools\/\?q=/.test(href)) problems.push(`${t.id}: ${href}`);
    }
  }
  assert.deepEqual(problems, []);
});

test('a link that opens the worked example says so with #example', () => {
  // Search engines ignore a fragment, so an example link cannot split the page.
  const home = readFileSync(join(dist, 'index.html'), 'utf8');
  // The instrument panel's readouts open each tool on its worked example.
  const links = [...home.matchAll(/<li class="gauge"><a href="([^"]+)"/g)].map((m) => m[1]);
  assert.ok(links.length >= 6, 'the home page links to no worked examples');
  for (const href of links) assert.match(href, /#example$/, href);
  const app = readFileSync(join(web, 'src/components/ToolApp.svelte'), 'utf8');
  // The page uses the same fragment when it puts the example back.
  assert.match(app, /'#example'/);
});

test('every canonical is the page it sits on, or its parent', () => {
  const problems = [];
  for (const t of catalog.tools) {
    const html = page(route(t.id));
    const canonical = canonicalOf(html);
    const self = `https://geoprims.com${route(t.id)}`;
    if (canonical === self) continue;
    // A generated endpoint points at the tool it presets.
    const parent = t.parent ?? t.composedOf[0];
    if (!parent) problems.push(`${t.id}: canonical ${canonical} is neither itself nor a parent`);
    else if (canonical !== `https://geoprims.com${route(parent)}`) {
      problems.push(`${t.id}: canonical ${canonical}, parent ${parent}`);
    }
  }
  assert.deepEqual(problems, []);
});

test('the gate bites', () => {
  assert.equal(canonicalOf('<link rel="canonical" href="https://geoprims.com/a/">'), 'https://geoprims.com/a/');
  assert.equal(canonicalOf('<link rel="alternate" href="x">'), undefined);
});
