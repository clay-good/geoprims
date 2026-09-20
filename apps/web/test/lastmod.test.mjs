// The sitemap lastmod ledger (add-seo-and-discoverability): a page's date moves
// when its words move, and not when the build merely renames a bundle.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { contentHash, substantive } from '../scripts/lastmod.mjs';

const web = new URL('..', import.meta.url).pathname;
const dist = join(web, 'dist');
const page = (route) => readFileSync(join(dist, route, 'index.html'), 'utf8');

test('a rebuilt island bundle does not change a page hash', () => {
  const html = page('/aviation/altimetry/density-altitude/');
  assert.match(html, /\/_astro\/ToolApp\.[A-Za-z0-9_-]{8}\.js/, 'the page really does carry a hashed island URL');
  const rebuilt = html.replace(/(\/_astro\/[\w.-]*?)\.[A-Za-z0-9_-]{8}\.(js|css)\b/g, '$1.aB3-dE7g.$2');
  assert.equal(contentHash(rebuilt), contentHash(html));
});

test('a rebuilt island id does not change a page hash', () => {
  const html = page('/aviation/altimetry/density-altitude/');
  assert.match(html, /<astro-island uid="[A-Za-z0-9]+"/, 'the page really does carry an island id');
  const rebuilt = html.replace(/<astro-island uid="[A-Za-z0-9]+"/g, '<astro-island uid="anotherId"');
  assert.equal(contentHash(rebuilt), contentHash(html));
});

test('hydration markers do not change a page hash', () => {
  const html = page('/aviation/altimetry/density-altitude/');
  const rebuilt = html.replace('<div class="value"', '<!--[-1--><!--]--><div class="value"');
  assert.notEqual(rebuilt, html);
  assert.equal(contentHash(rebuilt), contentHash(html));
});

test('a rebuilt core does not change a page hash', () => {
  const html = page('/aviation/altimetry/density-altitude/');
  assert.match(html, /buildHash&quot;:\[0,&quot;[0-9a-f]{8,}/, 'the island really does carry the build hash');
  const rebuilt = html.replace(/(buildHash&quot;:\[0,&quot;)[0-9a-f]{8,}/g, '$1beefbeefbeefbeef');
  assert.equal(contentHash(rebuilt), contentHash(html));
});

test('changed words do change the hash', () => {
  const html = page('/aviation/altimetry/density-altitude/');
  assert.notEqual(contentHash(html.replaceAll('Density altitude', 'Density height')), contentHash(html));
});

test('only the main region counts, so chrome and the footer are ignored', () => {
  const html = page('/aviation/altimetry/density-altitude/');
  assert.ok(!substantive(html).includes('<footer'));
  assert.equal(contentHash(html.replace('</main>', '</main><p>added after main</p>')), contentHash(html));
});

test('every page in the ledger is an indexable page of this build', () => {
  const ledger = JSON.parse(readFileSync(join(web, '../../data/seo/lastmod.json'), 'utf8'));
  for (const [path, entry] of Object.entries(ledger)) {
    assert.match(entry.lastmod, /^\d{4}-\d{2}-\d{2}$/, path);
    assert.equal(entry.hash, contentHash(page(path)), `${path} is stale: rebuild and commit data/seo/lastmod.json`);
  }
});
