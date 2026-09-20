import assert from 'node:assert/strict';
import { readdirSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { serveBuiltSite, web } from './site.mjs';

const path = '/assets/egm96-15/2009-08-29/egm96-15.pgm';
const input = JSON.stringify({ lat: 16.776, lon: -3.009 });

test('a corrupt service-worker asset is evicted and the retry downloads verified bytes', { timeout: 90_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const page = await browser.newPage();
  await page.goto(`${origin}/geodesy/geoid/geoid-height/`);
  await page.evaluate(async () => {
    await navigator.serviceWorker.register('/sw.js');
    await navigator.serviceWorker.ready;
    if (!navigator.serviceWorker.controller) {
      await new Promise((resolve) => navigator.serviceWorker.addEventListener('controllerchange', resolve, { once: true }));
    }
  });
  const appCache = await page.evaluate(async (asset) => {
    const name = (await caches.keys()).find((key) => key.startsWith('gp-app-'));
    if (!name) throw new Error('the release cache was not installed');
    const cache = await caches.open(name);
    if (!await cache.match(asset)) throw new Error('the geoid asset was not precached');
    await cache.put(asset, new Response(new Uint8Array([1, 2, 3])));
    return name;
  }, path);

  const workerFile = readdirSync(join(web, 'dist/_astro')).find((name) => /^compute\.worker-.*\.js$/.test(name));
  assert.ok(workerFile, 'the build has no browser compute worker');
  await page.evaluate((name) => { window.assetWorker = new Worker(`/_astro/${name}`, { type: 'module' }); }, workerFile);
  const invoke = () => page.evaluate((args) => new Promise((resolve, reject) => {
    const worker = window.assetWorker;
    worker.onmessage = ({ data }) => resolve(JSON.parse(data.out));
    worker.onerror = (event) => reject(new Error(event.message));
    worker.postMessage({ seq: 1, method: 'invoke', args });
  }), ['geodesy.geoid.geoid-height', input]);

  const first = await invoke();
  assert.equal(first.error.code, 'ASSET_INTEGRITY');
  assert.equal(await page.evaluate(async ({ name, asset }) => Boolean(await (await caches.open(name)).match(asset)), { name: appCache, asset: path }), false);

  const second = await invoke();
  assert.equal(second.ok, true, JSON.stringify(second));
  assert.ok(Math.abs(second.result.geoid_height.value - 28.7079) < 1e-4);
  assert.equal(await page.evaluate(async (asset) => Boolean(await (await caches.open('gp-pack-verified-assets')).match(asset)), path), true);

  await page.evaluate((name) => {
    window.assetWorker.terminate();
    window.assetWorker = new Worker(`/_astro/${name}`, { type: 'module' });
  }, workerFile);
  await page.context().setOffline(true);
  const offline = await invoke();
  assert.equal(offline.ok, true, 'the verified replacement was not available offline');
});
