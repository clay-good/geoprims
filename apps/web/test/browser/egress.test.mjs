import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { serveBuiltSite } from './site.mjs';

const root = fileURLToPath(new URL('../../../..', import.meta.url));
const tools = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8')).tools;

test('sentinel inputs from every tool stay out of browser requests', { timeout: 180_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const context = await browser.newContext({ serviceWorkers: 'block' });
  const sentinels = new Set();
  const leaks = [];
  const external = [];
  let wasmRequests = 0;
  await context.route('**/*', async (route) => {
    const request = route.request();
    const url = request.url();
    const text = [url, JSON.stringify(request.headers()), request.postDataBuffer()?.toString('utf8') ?? ''].join('\n');
    for (const sentinel of sentinels) {
      if (text.includes(sentinel) || text.includes(encodeURIComponent(sentinel))) leaks.push({ sentinel, url });
    }
    if (url.endsWith('.wasm')) wasmRequests++;
    if (new URL(url).origin !== origin) external.push(url);
    await route.continue();
  });
  const page = await context.newPage();

  // Prove the capture includes requests made by page code, not just navigations.
  const probe = 'ZZQQEGRESSPROBE9187';
  sentinels.add(probe);
  await page.goto(`${origin}/offline/`);
  await page.evaluate((url) => fetch(url), `${origin}/offline/?probe=${probe}`);
  assert.ok(leaks.some((entry) => entry.sentinel === probe), 'the proxy missed an injected leaking request');
  sentinels.delete(probe);
  leaks.length = 0;

  for (const [index, tool] of tools.entries()) {
    const route = `/${tool.id.split('.').join('/')}/`;
    const response = await page.goto(`${origin}${route}`, { waitUntil: 'domcontentloaded' });
    assert.equal(response.status(), 200, `${tool.id} page did not load`);
    await page.waitForFunction(() => [...document.querySelectorAll('astro-island')]
      .some((island) => island.getAttribute('component-url')?.includes('ToolApp') && !island.hasAttribute('ssr')));
    const field = page.locator('astro-island input[id^="field-"], astro-island textarea[id^="field-"]').first();
    assert.ok(await field.count(), `${tool.id} has no editable text input`);
    await field.evaluate((element) => { if (element.closest('details')) element.closest('details').open = true; });
    const sentinel = `ZZQQEGRESS${index}X9187`;
    sentinels.add(sentinel);
    await field.fill(sentinel);
    await page.waitForTimeout(200); // past the form's 150 ms recompute debounce
  }
  await page.waitForTimeout(500);
  assert.ok(wasmRequests > 0, 'the proxy did not capture requests from the compute worker');
  assert.deepEqual(leaks, [], 'a user input reached a request URL, header, or body');
  assert.deepEqual(external, [], 'a tool page made a third-party request');
  t.diagnostic(`${tools.length} tool pages checked with unique sentinel inputs`);
});
