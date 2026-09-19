// Offline PWA (web/offline-pwa): the manifest, the precache, and the service
// worker's install, offline, update, and cache-hygiene behavior, run in a
// simulated worker scope over the built site.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import vm from 'node:vm';
import { brotliCompressSync } from 'node:zlib';

const web = new URL('..', import.meta.url).pathname;
const dist = join(web, 'dist');
const catalog = JSON.parse(readFileSync(join(web, '../../dist/catalog/v1.json'), 'utf8'));
const ORIGIN = 'https://geoprims.com';
const route = (id) => '/' + id.split('.').join('/') + '/';

/** A served file for a site path, or null. */
function served(path) {
  const file = join(dist, path.endsWith('/') ? `${path}index.html` : path);
  return existsSync(file) && statSync(file).isFile() ? readFileSync(file) : null;
}

/** Loads dist/sw.js into a fake worker scope with Cache Storage and a switchable network. */
function worker(source = readFileSync(join(dist, 'sw.js'), 'utf8'), shared = { caches: new Map() }) {
  const listeners = {};
  const net = { online: true, requests: [] };
  const store = shared.caches;
  const key = (r) => new URL(typeof r === 'string' ? r : r.url, ORIGIN).href;
  const cacheOf = (name) => {
    if (!store.has(name)) store.set(name, new Map());
    const m = store.get(name);
    return {
      match: async (r) => m.get(key(r))?.clone() ?? undefined,
      put: async (r, res) => void m.set(key(r), res),
      addAll: async (reqs) => {
        for (const r of reqs) {
          const res = await scope.fetch(r);
          if (!res.ok) throw new TypeError(`${r.url} ${res.status}`);
          m.set(key(r), res);
        }
      },
    };
  };
  // Relative request URLs resolve against the worker's origin, as in a browser.
  const WorkerRequest = class extends Request {
    constructor(url, init) {
      super(typeof url === 'string' ? new URL(url, ORIGIN).href : url, init);
    }
  };
  const scope = {
    location: new URL(`${ORIGIN}/sw.js`),
    URL, Request: WorkerRequest, Response, Promise, console, JSON,
    skipped: false,
    skipWaiting() { this.skipped = true; },
    clients: { claim: async () => {} },
    caches: {
      open: async (name) => cacheOf(name),
      keys: async () => [...store.keys()],
      delete: async (name) => store.delete(name),
    },
    fetch: async (r) => {
      const url = new URL(typeof r === 'string' ? r : r.url, ORIGIN);
      net.requests.push(url.pathname);
      if (!net.online) throw new TypeError('Failed to fetch');
      const body = served(url.pathname);
      return body ? new Response(body, { status: 200 }) : new Response('not found', { status: 404 });
    },
    addEventListener: (type, fn) => (listeners[type] = fn),
  };
  scope.self = scope;
  vm.runInNewContext(source, scope);
  const dispatch = async (type, extra = {}) => {
    const waits = [];
    let response;
    listeners[type]({ ...extra, waitUntil: (p) => waits.push(p), respondWith: (p) => (response = p) });
    await Promise.all(waits);
    return response;
  };
  return {
    scope, net, store,
    install: () => dispatch('install'),
    activate: () => dispatch('activate'),
    message: (data) => dispatch('message', { data }),
    get: async (path, mode = 'no-cors') => {
      const request = new Request(`${ORIGIN}${path}`);
      const r = await dispatch('fetch', { request: { url: request.url, method: 'GET', mode } });
      return r ?? scope.fetch(request);
    },
  };
}

test('the web app manifest is installable', () => {
  const m = JSON.parse(readFileSync(join(dist, 'manifest.webmanifest'), 'utf8'));
  for (const k of ['name', 'short_name', 'start_url', 'scope', 'theme_color', 'background_color']) assert.ok(m[k], k);
  assert.equal(m.display, 'standalone');
  const png = (src) => {
    const b = readFileSync(join(dist, src));
    assert.equal(b.toString('latin1', 1, 4), 'PNG', src);
    return `${b.readUInt32BE(16)}x${b.readUInt32BE(20)}`;
  };
  for (const icon of m.icons.filter((i) => i.type === 'image/png')) assert.equal(png(icon.src), icon.sizes, icon.src);
  assert.ok(m.icons.some((i) => i.purpose === 'maskable' && i.sizes === '512x512'));
  assert.ok(m.icons.some((i) => i.purpose === 'any' && i.sizes === '192x192'));
  for (const s of m.shortcuts) assert.ok(served(s.url), s.url);
  const home = readFileSync(join(dist, 'index.html'), 'utf8');
  assert.match(home, /<link rel="manifest" href="\/manifest.webmanifest"/);
  assert.match(home, /<link rel="apple-touch-icon" href="\/icons\/apple-touch-icon.png"/);
});

test('the precache holds every page, module, and asset, within 12 MB compressed', () => {
  const precache = vm.runInNewContext(`${readFileSync(join(dist, 'sw.js'), 'utf8').split('\n\n')[0]}; PRECACHE`);
  const set = new Set(precache);
  for (const t of catalog.tools) assert.ok(set.has(route(t.id)), `${t.id} page`);
  for (const f of readdirSync(join(dist, 'wasm'))) assert.ok(set.has(`/wasm/${f}`), f);
  for (const u of ['/', '/offline/', '/catalog/v1.json', '/manifest.webmanifest', '/methodology/']) assert.ok(set.has(u), u);
  const registry = JSON.parse(readFileSync(join(dist, 'assets/registry.json'), 'utf8'));
  for (const a of registry.assets.filter((x) => x.loadPolicy === "on-demand")) for (const f of Object.keys(a.files)) assert.ok(set.has(`/assets/${a.id}/${a.version}/${f}`), f);
  assert.ok(!precache.some((u) => u.startsWith('/vectors/') || u === '/sw.js'), 'downloads and the worker itself are not precached');
  let bytes = 0;
  for (const u of precache) {
    const body = served(u);
    assert.ok(body, `${u} is not in the build`);
    bytes += brotliCompressSync(body).length;
  }
  assert.ok(bytes <= 12_000_000, `${bytes} bytes`);
});

test('airplane mode: after one visit, tool pages, modules, and assets load offline', async () => {
  const sw = worker();
  await sw.install();
  await sw.activate();
  sw.net.online = false;
  sw.net.requests.length = 0;
  const page = await sw.get('/aviation/altimetry/density-altitude/', 'navigate');
  assert.equal(page.status, 200);
  assert.match(await page.text(), /Density altitude/);
  assert.equal((await sw.get('/aviation/altimetry/density-altitude', 'navigate')).status, 200, 'without the trailing slash');
  const wasm = await sw.get('/wasm/aviation.wasm');
  assert.deepEqual(Buffer.from(await wasm.arrayBuffer()).subarray(0, 4), Buffer.from([0, 0x61, 0x73, 0x6d]));
  assert.equal((await sw.get('/catalog/v1.json')).status, 200);
  assert.equal((await sw.get('/assets/registry.json')).status, 200);
  const unknown = await sw.get('/no/such/page/', 'navigate');
  assert.match(await unknown.text(), /You(&#39;|')re offline/);
  assert.deepEqual(sw.net.requests.filter((p) => p !== '/no/such/page/'), [], 'nothing else touched the network');
});

test('updates wait for the user, and activation keeps offline packs', async () => {
  const shared = { caches: new Map() };
  const v1 = worker(undefined, shared);
  await v1.install();
  await v1.activate();
  shared.caches.set('gp-pack-magnetic-wmm2025', new Map());
  const source = readFileSync(join(dist, 'sw.js'), 'utf8').replace(/^const VERSION = '[0-9a-f]+';/, "const VERSION = 'next0000000';");
  const v2 = worker(source, shared);
  await v2.install();
  assert.equal(v2.scope.skipped, false, 'a new release does not take over on its own');
  const names = () => [...shared.caches.keys()].sort();
  assert.equal(names().filter((n) => n.startsWith('gp-app-')).length, 2, 'the old release stays cached until activation');
  await v2.message({ type: 'SKIP_WAITING' });
  assert.equal(v2.scope.skipped, true, '"Reload to update" activates it');
  await v2.activate();
  assert.deepEqual(names(), ['gp-app-next0000000', 'gp-pack-magnetic-wmm2025']);
});

test('pages show the release version and offer updates without blocking', () => {
  const version = /const VERSION = '([0-9a-f]+)'/.exec(readFileSync(join(dist, 'sw.js'), 'utf8'))[1];
  const html = readFileSync(join(dist, 'aviation/altimetry/density-altitude/index.html'), 'utf8');
  assert.ok(html.includes(`<span data-app-version>${version}</span>`), 'footer version');
  assert.ok(html.includes(`core ${catalog.coreVersion}`));
  assert.match(html, /<div class="update card" role="status" hidden>/);
  assert.ok(!html.includes('__GP_APP_VERSION__'));
});
