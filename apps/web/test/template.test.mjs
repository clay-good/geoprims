// The page template (openspec web/page-template) on the built site
// (run `npm run build` in apps/web first).
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { matches } from '../src/lib/filter.js';

const web = new URL('..', import.meta.url).pathname;
const dist = join(web, 'dist');
const catalog = JSON.parse(readFileSync(join(web, '../../dist/catalog/v1.json'), 'utf8'));
const page = (route) => readFileSync(join(dist, route, 'index.html'), 'utf8');
const route = (id) => '/' + id.split('.').join('/') + '/';

function htmlFiles(dir, out = []) {
  for (const f of readdirSync(dir)) {
    const p = join(dir, f);
    if (statSync(p).isDirectory()) htmlFiles(p, out);
    else if (f.endsWith('.html')) out.push(p);
  }
  return out;
}

test('every page has one page header with exactly one h1', () => {
  for (const f of htmlFiles(dist)) {
    const html = readFileSync(f, 'utf8');
    assert.equal(html.match(/<h1[\s>]/g)?.length, 1, f.slice(dist.length));
    // The home hero is its own header; every other page uses PageHeader.
    if (f !== join(dist, 'index.html')) assert.match(html, /<header class="page-head"/, f.slice(dist.length));
  }
});

test('lists of more than eight tools have a filter; topic pages have jump chips', () => {
  assert.match(page('/geodesy/'), /aria-label="Jump to a group"/);
  const units = page('/units/');
  assert.match(units, /class="filter"/);
  assert.match(units, /class="filter-empty card" hidden/);
  assert.match(units, /data-palette=""/);
  const items = [...units.matchAll(/<li data-text="([^"]+)"/g)].map((m) => m[1]);
  assert.ok(items.length > 8);
  assert.ok(items.some((t) => matches(t, 'knots mph')), 'aliases and keywords are searchable');
});

test('the filter matches every word, in any order and case', () => {
  assert.ok(matches('crosswind and headwind components', 'Wind CROSS'));
  assert.ok(matches('anything', '  '));
  assert.ok(!matches('crosswind', 'crosswind tailwind'));
});

test('tools with an inverse link to it from the header, and related reasons read as words', () => {
  const t = catalog.tools.find((x) => x.id === 'aviation.airspeed.cas-to-tas');
  const html = page(route(t.id));
  assert.match(html, /class="other-way"><a href="\/aviation\/airspeed\/tas-to-cas\/">Go the other way: /);
  assert.doesNotMatch(html, /<p>(inverse|next|alternative|parent)<\/p>/);
});

test('unknown URLs get a not-found page with search and popular tools', () => {
  const html = readFileSync(join(dist, '404.html'), 'utf8');
  assert.match(html, /We couldn’t find that page/);
  assert.match(html, /class="hero-search" data-palette/);
  assert.match(html, /<meta name="robots" content="noindex">/);
});

test('home offers example searches that open the palette', () => {
  const html = page('/');
  assert.ok([...html.matchAll(/class="chip-button" data-palette="[^"]+"/g)].length >= 4);
});
