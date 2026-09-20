// geoprims service worker (web/offline-pwa). scripts/pwa.mjs prepends
// `const VERSION = '…'` and `const PRECACHE = […]` (every page, module, the
// catalog, and data assets) at build time.
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
});

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
        return await fetch(req);
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
