import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { join, relative } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { captureCsp, serveBuiltSite, web } from './site.mjs';

const root = fileURLToPath(new URL('../../../..', import.meta.url));
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const toolRoutes = new Set(catalog.tools.map((tool) => `${tool.id.split('.').join('/')}/index.html`));
const dist = join(web, 'dist');

function htmlFiles(dir, found = []) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) htmlFiles(path, found);
    else if (entry.name.endsWith('.html')) found.push(relative(dist, path));
  }
  return found;
}

test('production CSP reports no violations on non-tool routes and the report dialog', { timeout: 180_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const context = await browser.newContext({ serviceWorkers: 'block' });
  const violations = await captureCsp(context);
  const external = [];
  context.on('request', (request) => {
    if (new URL(request.url()).origin !== origin) external.push(request.url());
  });
  const page = await context.newPage();

  // An injected inline script must be blocked and must appear in the report.
  await page.goto(`${origin}/offline/`);
  await page.evaluate(() => {
    const script = document.createElement('script');
    script.textContent = 'window.__cspProbe = true';
    document.head.append(script);
  });
  await page.waitForTimeout(100);
  assert.equal(await page.evaluate(() => window.__cspProbe), undefined, 'the CSP allowed an injected inline script');
  assert.ok(violations.some((v) => v.directive.startsWith('script-src')), 'the CSP reporter missed a blocked script');
  violations.length = 0;

  const otherRoutes = htmlFiles(dist).filter((path) => !toolRoutes.has(path));
  for (const path of otherRoutes) {
    const route = path === '404.html' ? '/missing-csp-fixture/' : path === 'index.html' ? '/' : `/${path.replace(/index\.html$/, '')}`;
    const response = await page.goto(`${origin}${route}`, { waitUntil: 'load' });
    assert.equal(response.status(), path === '404.html' ? 404 : 200, `${route} failed to load`);
  }
  // The footer's page report on a non-tool page, then the tool's own report.
  await page.goto(`${origin}/privacy/`);
  await page.locator('footer [data-report]').click();
  await page.locator('dialog[open]').waitFor();
  await page.waitForTimeout(100);
  assert.deepEqual(external, [], 'a page report made a third-party request while paused');
  await page.getByRole('button', { name: 'Close' }).click();
  await page.goto(`${origin}/aviation/altimetry/density-altitude/`);
  await page.locator('.report-button').click();
  await page.locator('dialog[open]').waitFor();
  await page.waitForTimeout(100);
  assert.deepEqual(external, [], 'a non-tool page made a third-party request');
  await page.getByRole('button', { name: 'Close' }).click();

  // An enabled report may load the one allowed third-party script, and only
  // after the user opens the dialog. Stub its contents to keep the test local.
  await context.route('**/api/reports/config', (route) => route.fulfill({
    status: 200, contentType: 'application/json', body: JSON.stringify({ enabled: true, sitekey: 'test' }),
  }));
  await context.route(/^https:\/\/challenges\.cloudflare\.com\/turnstile\/v0\/api\.js/, (route) => route.fulfill({
    status: 200, contentType: 'text/javascript', body: 'window.turnstile={render(){return "test"},reset(){}};',
  }));
  await page.locator('.report-button').click();
  await page.waitForFunction(() => document.querySelector('dialog button[type="submit"]')?.disabled === false);
  assert.equal(external.length, 1, `the dialog made ${external.length} third-party requests`);
  assert.match(external[0], /^https:\/\/challenges\.cloudflare\.com\/turnstile\/v0\/api\.js/);
  assert.deepEqual(violations, [], 'a page or the report dialog violated the production CSP');
  t.diagnostic(`${otherRoutes.length} non-tool routes and the report dialog checked`);
});
