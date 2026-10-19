import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { captureCsp, serveBuiltSite } from './site.mjs';

const root = fileURLToPath(new URL('../../../..', import.meta.url));
const tools = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8')).tools;

// A privacy check, not a speed check: the budget grows with the catalog
// (about half a second a page on a quiet machine), so a new tool never makes
// it fail for time.
test('sentinel inputs from every tool stay out of browser requests', { timeout: Math.max(180_000, tools.length * 1_000) }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const context = await browser.newContext({ serviceWorkers: 'block' });
  const violations = await captureCsp(context);
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
    const hasInputs = Object.keys(tool.inputs?.properties ?? {}).some((key) => key !== 'options');
    if (!hasInputs) {
      assert.equal(await field.count(), 0, `${tool.id} declares no inputs but renders an editable text input`);
      continue;
    }
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
  assert.deepEqual(violations, [], 'a tool page violated the production CSP');
  t.diagnostic(`${tools.length} tool pages checked with unique sentinel inputs`);
});

// Equipment profiles (web/equipment-profiles, scenario "Egress test"): a
// profile of every kind, named with a sentinel, is loaded on every tool that
// takes one, and the name reaches no request.
test('a sentinel profile name loaded on every profile-taking tool stays out of requests', { timeout: 300_000 }, async (t) => {
  const profiles = JSON.parse(readFileSync(join(root, 'data/profiles.json'), 'utf8'));
  const byId = new Map(tools.map((x) => [x.id, x]));
  const name = 'ZZQQPROFILE9187';
  const store = {};
  for (const [kind, def] of Object.entries(profiles.kinds)) {
    const saved = {};
    for (const [id, fields] of Object.entries(def.tools)) {
      const tool = byId.get(id);
      const ex = (tool.examples.find((e) => e.id === tool['x-primary-example']) ?? tool.examples[0]).input;
      saved[id] = Object.fromEntries(fields.filter((f) => ex[f] !== undefined).map((f) => [f, ex[f]]));
    }
    store[kind] = { [name]: { format: profiles.format, version: profiles.version, kind, name, savedOn: '2026-10-19', tools: saved } };
  }
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const context = await browser.newContext({ serviceWorkers: 'block' });
  await context.addInitScript((s) => localStorage.setItem('gp-profiles', s), JSON.stringify(store));
  const leaks = [];
  await context.route('**/*', async (route) => {
    const request = route.request();
    const text = [request.url(), JSON.stringify(request.headers()), request.postDataBuffer()?.toString('utf8') ?? ''].join('\n');
    if (text.includes(name)) leaks.push(request.url());
    await route.continue();
  });
  const page = await context.newPage();
  let loaded = 0;
  for (const [kind, def] of Object.entries(profiles.kinds)) {
    for (const id of Object.keys(def.tools)) {
      await page.goto(`${origin}/${id.split('.').join('/')}/`, { waitUntil: 'domcontentloaded' });
      await page.waitForFunction(() => [...document.querySelectorAll('astro-island')]
        .some((island) => island.getAttribute('component-url')?.includes('ToolApp') && !island.hasAttribute('ssr')));
      await page.locator('summary', { hasText: `${def.title} profiles` }).click();
      await page.getByRole('button', { name: 'Load', exact: true }).click();
      // A tool whose worked example sets none of the kept fields has nothing to load.
      const status = await page.locator('.profiles [role="status"]').textContent();
      if (/nothing saved/.test(status)) continue;
      await page.locator('.profile-chip').waitFor({ timeout: 5_000 }).catch(() => assert.fail(`${id}: no profile chip after Load (${status})`));
      await page.waitForTimeout(200); // past the form's recompute debounce
      loaded++;
    }
  }
  assert.ok(loaded >= 20, `only ${loaded} profile loads`);
  assert.deepEqual(leaks, [], 'a profile name reached a request');
  t.diagnostic(`${loaded} profile loads checked`);
});

