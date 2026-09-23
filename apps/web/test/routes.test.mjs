// The route-map gate (contracts/routes-and-urls). Checks the built site, then
// the classifier against paths the contract does and does not allow.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { check, checkRedirects, classify, deprecationRedirects, index, serialize, SITE } from '../scripts/routes.mjs';

const web = new URL('..', import.meta.url).pathname;
const dist = join(web, 'dist');
const catalog = JSON.parse(readFileSync(join(web, '../../dist/catalog/v1.json'), 'utf8'));
const idx = index(catalog);
const page = (route) => readFileSync(join(dist, route, 'index.html'), 'utf8');
const ok = (route) => ({ route, html: page(route) });

test('every route the contract allows classifies', () => {
  assert.equal(classify('/', idx), 'home');
  assert.equal(classify('/tools/', idx), 'catalog');
  assert.equal(classify('/methodology/', idx), 'trust');
  assert.equal(classify('/accuracy/', idx), 'trust');
  assert.equal(classify('/verification/0.1.0/', idx), 'verification');
  assert.equal(classify('/offline/', idx), 'app');
  assert.equal(classify('/aviation/', idx), 'domain');
  assert.equal(classify('/aviation/altimetry/', idx), 'group');
  assert.equal(classify('/aviation/altimetry/density-altitude/', idx), 'tool');
  assert.equal(classify('/learn/what-is-density-altitude/', idx), 'explainer');
  assert.equal(classify('/journeys/vfr-preflight/', idx), 'journey');
});

test('a page outside the route map fails the gate, naming the path', () => {
  assert.equal(classify('/tools/density-altitude/', idx), null);
  const problems = check([{ route: '/tools/density-altitude/', html: '' }], idx);
  assert.equal(problems.length, 1);
  assert.match(problems[0], /^\/tools\/density-altitude\/ is not a route/);
});

test('URLs that break the stable-URL rules are outside the map', () => {
  for (const bad of ['/Aviation/', '/aviation/altimetry/density_altitude/', '/aviation/altimetry/density-altitude']) {
    assert.equal(classify(bad, idx), null, bad);
  }
});

test('a generated endpoint must canonicalize to the operation it composes', () => {
  const endpoint = catalog.tools.find((t) => t.composedOf.length > 0);
  const route = '/' + endpoint.id.split('.').join('/') + '/';
  const parent = `${SITE}/${endpoint.composedOf[0].split('.').join('/')}/`;
  assert.deepEqual(check([ok(route)], idx), []);
  assert.match(page(route), new RegExp(`<link rel="canonical" href="${parent.replaceAll('/', '\\/')}"`));
  const wrong = page(route).replace(/<link rel="canonical" href="[^"]*"/, `<link rel="canonical" href="${SITE}/"`);
  assert.match(check([{ route, html: wrong }], idx)[0], /must canonicalize to/);
});

test('app routes must carry noindex', () => {
  assert.deepEqual(check([ok('/offline/')], idx), []);
  assert.match(check([{ route: '/offline/', html: '' }], idx)[0], /must carry noindex/);
});

test('the whole built site is inside the route map', () => {
  assert.deepEqual(check([ok('/'), ok('/tools/'), ok('/aviation/'), ok('/aviation/altimetry/'), ok('/aviation/altimetry/density-altitude/')], idx), []);
});

test('the redirects file keeps renamed routes resolvable', () => {
  const built = new Set(['/aviation/altimetry/density-altitude/']);
  const good = [{ from: '/aviation/da/', to: '/aviation/altimetry/density-altitude/', since: '2026-01-15' }];
  assert.deepEqual(checkRedirects(good, built), []);
  assert.equal(serialize(good), '/aviation/da/ /aviation/altimetry/density-altitude/ 301\n');
  assert.match(checkRedirects([{ ...good[0], to: '/aviation/gone/' }], built)[0], /is not a page this build emits/);
  assert.match(checkRedirects([{ ...good[0], from: '/aviation/altimetry/density-altitude/' }], built)[0], /shadows a page/);
  assert.match(checkRedirects([{ ...good[0], since: 'January' }], built).at(-1), /no ISO since date/);
});

test('the build writes the redirects file from data/redirects.json and the catalog', () => {
  const list = JSON.parse(readFileSync(join(web, '../../data/redirects.json'), 'utf8'));
  assert.ok(Array.isArray(list));
  assert.equal(readFileSync(join(dist, '_redirects'), 'utf8'), serialize([...list, ...deprecationRedirects(catalog)]));
});

test('a deprecated tool redirects to its replacement and leaves the index', () => {
  // No tool is deprecated yet, so the rules are checked on a fixture.
  assert.deepEqual(deprecationRedirects(catalog), [], 'nothing is deprecated today');
  const gone = {
    id: 'aviation.wind.old-crosswind',
    deprecation: { replacement: 'aviation.wind.runway-components', removal: '2.0.0' },
  };
  assert.deepEqual(deprecationRedirects({ tools: [...catalog.tools, gone] }, '2026-09-20'), [
    { from: '/aviation/wind/old-crosswind/', to: '/aviation/wind/runway-components/', since: '2026-09-20', deprecated: true },
  ]);
  // Its own page is still built until removal, so the redirect may name it.
  const built = new Set(['/aviation/wind/old-crosswind/', '/aviation/wind/runway-components/']);
  assert.deepEqual(checkRedirects(deprecationRedirects({ tools: [gone] }, '2026-09-20'), built), []);
  // A plain rename may not shadow a live page.
  assert.match(
    checkRedirects([{ from: '/aviation/wind/old-crosswind/', to: '/aviation/wind/runway-components/', since: '2026-09-20' }], built)[0],
    /shadows a page/,
  );
});
