// geoprims service worker (web/offline-pwa). scripts/pwa.mjs prepends
// `const VERSION = '…'` and `const PRECACHE = […]` (every page, module, the
// catalog, and calculation data assets) at build time. Optional map detail is
// cached the first time it is requested.
//
// - Install caches the whole release, then waits: a new release never takes
//   over mid-session. It activates when every tab has closed, or when the page
//   posts SKIP_WAITING after the user accepts "Reload to update".
// - Requests are answered from the active release's cache first, so results
//   never change until the new version activates. The old release stays
//   cached until then.
// - Activation deletes older release caches and keeps offline packs
//   (`gp-pack-*`), which outlive releases.

const APP = `gp-app-${VERSION}`;
const PACK_PREFIX = 'gp-pack-';
const PACK_MARK = '/_gp-pack-complete';
const OFFLINE = '/offline/';

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(APP).then((cache) => cache.addAll(PRECACHE.map((url) => new Request(url, { cache: 'reload' })))),
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k.startsWith('gp-app-') && k !== APP).map((k) => caches.delete(k))))
      .then(() => self.clients.claim()),
  );
});

self.addEventListener('message', (event) => {
  if (event.data?.type === 'SKIP_WAITING') self.skipWaiting();
  if (event.data?.type === 'VERSION') event.source?.postMessage({ type: 'VERSION', version: VERSION });
  if (event.data?.type?.startsWith('PACK_')) event.waitUntil(handlePackMessage(event));
});

const packCache = (pack) => `${PACK_PREFIX}${pack.id}-${pack.version}`;
const packFor = (id) => PACKS.find((pack) => pack.id === id);
const hex = (bytes) => [...new Uint8Array(bytes)].map((byte) => byte.toString(16).padStart(2, '0')).join('');

async function verified(response, file) {
  if (!response?.ok) return false;
  const body = await response.arrayBuffer();
  if (body.byteLength !== file.bytes) return false;
  return hex(await crypto.subtle.digest('SHA-256', body)) === file.sha256;
}

async function packStatus() {
  const names = await caches.keys();
  const states = [];
  for (const pack of PACKS) {
    const prefix = `${PACK_PREFIX}${pack.id}-`;
    const currentName = packCache(pack);
    const versions = [];
    for (const name of names.filter((candidate) => candidate.startsWith(prefix))) {
      const mark = await (await caches.open(name)).match(PACK_MARK);
      if (mark) versions.push(name.slice(prefix.length));
    }
    states.push({
      id: pack.id,
      version: pack.version,
      installed: versions.includes(pack.version),
      installedVersions: versions,
      updateAvailable: versions.length > 0 && !versions.includes(pack.version),
      partial: names.includes(currentName) && !versions.includes(pack.version),
    });
  }
  return states;
}

async function installPack(pack, send) {
  const name = packCache(pack);
  const cache = await caches.open(name);
  let done = 0;
  for (const file of pack.files) {
    const cached = await cache.match(file.path);
    if (cached && await verified(cached.clone(), file)) {
      done += file.bytes;
      send({ type: 'PACK_PROGRESS', id: pack.id, done, total: pack.bytes });
      continue;
    }
    if (cached) await cache.delete(file.path);
    const response = await fetch(new Request(file.path, { cache: 'reload' }));
    if (!response.ok) throw new Error(`Download failed (${response.status})`);
    const body = await response.arrayBuffer();
    if (body.byteLength !== file.bytes || hex(await crypto.subtle.digest('SHA-256', body)) !== file.sha256) {
      throw new Error('Downloaded file did not match its published checksum');
    }
    await cache.put(file.path, new Response(body, { status: 200, headers: response.headers }));
    done += file.bytes;
    send({ type: 'PACK_PROGRESS', id: pack.id, done, total: pack.bytes });
  }
  await cache.put(PACK_MARK, new Response(JSON.stringify({ id: pack.id, version: pack.version })));
  for (const old of (await caches.keys()).filter((candidate) => candidate.startsWith(`${PACK_PREFIX}${pack.id}-`) && candidate !== name)) {
    await caches.delete(old);
  }
}

async function handlePackMessage(event) {
  const port = event.ports?.[0] ?? event.source;
  const send = (data) => port?.postMessage(data);
  try {
    const { type, id } = event.data;
    const pack = id && packFor(id);
    if (id && !pack) throw new Error('Unknown offline pack');
    if (type === 'PACK_INSTALL') await installPack(pack, send);
    if (type === 'PACK_DELETE') {
      const prefix = `${PACK_PREFIX}${pack.id}-`;
      await Promise.all((await caches.keys()).filter((name) => name.startsWith(prefix)).map((name) => caches.delete(name)));
    }
    send({ type: 'PACK_RESULT', ok: true, packs: await packStatus() });
  } catch (error) {
    send({ type: 'PACK_RESULT', ok: false, error: error instanceof Error ? error.message : String(error), packs: await packStatus() });
  }
}

/** Cache key for a same-origin request: path only, with directory routes ending in "/". */
function keyOf(url) {
  let path = url.pathname;
  if (!path.endsWith('/') && !path.split('/').pop().includes('.')) path += '/';
  return path;
}

async function lookup(path) {
  const app = await caches.open(APP);
  const hit = await app.match(path);
  if (hit) return hit;
  for (const name of await caches.keys()) {
    if (!name.startsWith(PACK_PREFIX)) continue;
    const packed = await (await caches.open(name)).match(path);
    if (packed) return packed;
  }
  return null;
}

self.addEventListener('fetch', (event) => {
  const req = event.request;
  if (req.method !== 'GET') return;
  const url = new URL(req.url);
  if (url.origin !== self.location.origin) return;
  // The report API is never cached, replayed, or answered offline: a report
  // must reach the Worker or fail in the open, so the dialog can offer
  // "Copy report" instead (contracts/report-api).
  if (url.pathname.startsWith('/api/')) return;
  event.respondWith(
    (async () => {
      const path = keyOf(url);
      const cached = await lookup(path);
      if (cached) return cached;
      try {
        const response = await fetch(req);
        if (response.ok && path.startsWith('/assets/')) {
          await (await caches.open(APP)).put(path, response.clone());
        }
        return response;
      } catch (err) {
        if (req.mode === 'navigate') {
          const page = await lookup(OFFLINE);
          if (page) return page;
        }
        throw err;
      }
    })(),
  );
});
