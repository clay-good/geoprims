// Group hubs (web/tool-docs, "Domain and group index pages"): every group's
// tools are listed under the tasks they serve, each exactly once, and the
// airspeed hub carries the IAS → CAS → EAS → TAS → Mach guide.
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { catalog, hubFor } from '../src/lib/catalog.mjs';

const web = new URL('..', import.meta.url).pathname;
const hubs = JSON.parse(readFileSync(join(web, '../../data/hubs.json'), 'utf8')).hubs;
const groups = [...new Set(catalog.tools.map((t) => `${t.domain}.${t.group}`))];
const domains = [...new Set(catalog.tools.map((t) => t.domain))];

test('every group lists each of its tools under exactly one task', () => {
  for (const key of groups) {
    const [domain, group] = key.split('.');
    if (domain !== 'units') assert.ok(hubs[key], `${key}: no hub entry in data/hubs.json`);
    const hub = hubFor(domain, group);
    const listed = hub.tasks.flatMap((t) => t.tools.map((x) => x.id));
    const own = catalog.tools.filter((t) => t.domain === domain && t.group === group).map((t) => t.id);
    assert.deepEqual([...listed].sort(), [...own].sort(), `${key}: tasks must list every tool once`);
    for (const t of hub.tasks) assert.match(t.want, /^[a-z]/, `${key}: "${t.want}" reads after "I want to"`);
  }
  // A domain may carry its own entry (an intro for its hub page).
  for (const key of Object.keys(hubs)) assert.ok(groups.includes(key) || domains.includes(key), `${key}: no such group or domain`);
});

test('every tool a hub names exists, including guide steps elsewhere', () => {
  for (const [key, hub] of Object.entries(hubs)) {
    const [domain, group] = key.split('.');
    const refs = [...(hub.tasks ?? []).flatMap((t) => t.tools), ...(hub.guide?.steps ?? []).map((s) => s.tool).filter(Boolean)];
    for (const ref of refs) {
      const id = ref.includes('.') ? ref : `${domain}.${group}.${ref}`;
      assert.ok(catalog.tools.some((t) => t.id === id), `${key}: ${ref} is not a tool`);
    }
  }
});

test('group index: /aviation/airspeed lists every airspeed tool with its summary and the IAS → CAS → EAS → TAS → Mach guide', () => {
  const file = join(web, 'dist/aviation/airspeed/index.html');
  if (!existsSync(file)) return; // the build test covers a fresh build
  const html = readFileSync(file, 'utf8');
  const esc = (s) => s.replaceAll('&', '&amp;');
  for (const t of catalog.tools.filter((x) => x.domain === 'aviation' && x.group === 'airspeed')) {
    assert.ok(html.includes(`href="/aviation/airspeed/${t.id.split('.')[2]}/"`), `${t.id}: not linked`);
    assert.ok(html.includes(esc(t.summary)), `${t.id}: summary missing`);
  }
  assert.match(html, /IAS → CAS → EAS → TAS → Mach/);
  const guide = html.slice(html.indexOf('id="guide-title"'));
  const at = ['calibrated airspeed (CAS)', 'equivalent airspeed (EAS)', 'true airspeed (TAS)', 'Mach number'].map((w) => guide.indexOf(w));
  assert.ok(at.every((i) => i > 0) && at.every((i, k) => k === 0 || i > at[k - 1]), `guide order: ${at}`);
  assert.equal((html.match(/I want to<\/span>/g) ?? []).length, 3);
});

test('every domain and group hub carries a CollectionPage of exactly its tools, breadcrumbs, and working links', () => {
  // add-seo-and-discoverability 2.3: hub JSON-LD and links.
  const dist = join(web, 'dist');
  if (!existsSync(join(dist, 'aviation/index.html'))) return; // the build test covers a fresh build
  const SITE = 'https://geoprims.com';
  const url = (id) => `${SITE}/${id.split('.').join('/')}/`;
  const problems = [];
  const domains = [...new Set(catalog.tools.map((t) => t.domain))];
  const pages = [
    ...domains.map((d) => ({ path: `/${d}/`, tools: catalog.tools.filter((t) => t.domain === d), crumbs: 2 })),
    ...groups.map((k) => {
      const [d, g] = k.split('.');
      return { path: `/${d}/${g}/`, tools: catalog.tools.filter((t) => t.domain === d && t.group === g), crumbs: 3 };
    }),
  ];
  for (const { path, tools, crumbs } of pages) {
    const file = join(dist, path, 'index.html');
    if (!existsSync(file)) {
      problems.push(`${path}: no hub page`);
      continue;
    }
    const html = readFileSync(file, 'utf8');
    const ld = [...html.matchAll(/<script type="application\/ld\+json">([\s\S]*?)<\/script>/g)].map((m) => JSON.parse(m[1].replaceAll('\\u003c', '<')));
    const listed = (ld.find((o) => o['@type'] === 'CollectionPage')?.mainEntity?.itemListElement ?? []).map((i) => i.url);
    const want = tools.filter((t) => t.composedOf.length === 0).map((t) => url(t.id));
    if (JSON.stringify([...listed].sort()) !== JSON.stringify([...want].sort())) {
      problems.push(`${path}: the ItemList lists ${listed.length} tools, the hub has ${want.length}`);
    }
    const trail = ld.find((o) => o['@type'] === 'BreadcrumbList')?.itemListElement ?? [];
    if (trail.length !== crumbs || trail.at(-1)?.item !== `${SITE}${path}`) {
      problems.push(`${path}: breadcrumbs end at ${trail.at(-1)?.item} after ${trail.length} steps`);
    }
    // Every internal page link resolves to a built page.
    for (const [, href] of html.matchAll(/href="(\/[a-z0-9/-]*\/)(?:#[^"]*)?"/g)) {
      if (!existsSync(join(dist, href, 'index.html'))) problems.push(`${path}: link to ${href} has no page`);
    }
  }
  assert.deepEqual([...new Set(problems)], []);
  assert.ok(pages.length >= 60, `only ${pages.length} hubs checked`);
});

test('hub intros are a short paragraph of plain prose', () => {
  for (const [key, hub] of Object.entries(hubs)) {
    if (!hub.intro) continue;
    const words = hub.intro.split(/\s+/).length;
    assert.ok(words >= 30 && words <= 140, `${key}: intro is ${words} words, want 30 to 140`);
    assert.ok(!/[<>]/.test(hub.intro), `${key}: intro is plain text`);
  }
});

test('every group and domain hub has an intro, so no hub page is a bare list', () => {
  const missing = [...groups, ...domains].filter((key) => !hubs[key]?.intro);
  assert.deepEqual(missing, []);
});

