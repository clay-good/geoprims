// Checks the built site (run `npm run build` in apps/web first).
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';

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

test('one page per catalog endpoint', () => {
  for (const t of catalog.tools) assert.ok(existsSync(join(dist, route(t.id), 'index.html')), route(t.id));
  const toolPages = htmlFiles(dist).filter((p) => p.slice(dist.length).split('/').length === 5);
  assert.equal(toolPages.length, catalog.counts.all.endpoints);
});

test('every tool page carries the worked example answer in its HTML', () => {
  for (const t of catalog.tools) {
    const html = page(route(t.id));
    assert.match(html, /class="value[^"]*">[^<]+</, `${t.id}: no answer value`);
    assert.match(html, /class="sentence[^"]*">[^<]+</, `${t.id}: no sentence`);
  }
});

test('generated endpoints canonicalize to their parent; experimental pages are noindex', () => {
  const pair = page('/units/speed/kt-to-mph/');
  assert.match(pair, /<link rel="canonical" href="https:\/\/geoprims\.com\/units\/speed\/convert\/">/);
  for (const t of catalog.tools.filter((x) => x.stability === 'experimental')) {
    assert.match(page(route(t.id)), /<meta name="robots" content="noindex">/, t.id);
  }
});

test('no third-party scripts, styles, or fonts', () => {
  for (const f of htmlFiles(dist)) {
    const html = readFileSync(f, 'utf8');
    for (const m of html.matchAll(/<(script|link)[^>]+(src|href)="(https?:)?\/\/([^/"]+)/g)) {
      const tag = m[0];
      if (/rel="canonical"/.test(tag)) continue;
      assert.fail(`${f.slice(dist.length)} loads ${m[4]}`);
    }
  }
});

test('ships the Wasm modules and the catalog at their contract routes', () => {
  assert.ok(existsSync(join(dist, 'catalog/v1.json')));
  for (const m of ['base', 'link']) assert.ok(existsSync(join(dist, 'wasm', `${m}.wasm`)), m);
});

test('every aviation, drone, and navigation page shows the safety notice', () => {
  for (const t of catalog.tools.filter((x) => ['aviation', 'drone', 'navigation'].includes(x.domain))) {
    assert.match(page(route(t.id)), /Not certified for navigation/, t.id);
  }
});

test('every tool page has one report button and no bot-check script in its HTML', () => {
  for (const t of catalog.tools) {
    const html = page(route(t.id));
    assert.equal((html.match(/>Report a problem</g) ?? []).length, 1, t.id);
    assert.ok(!html.includes('challenges.cloudflare.com'), `${t.id} loads the bot check before a click`);
  }
});

test('the known-issues page is published', () => {
  assert.match(page('/known-issues/'), /<h1>Known issues<\/h1>/);
});
