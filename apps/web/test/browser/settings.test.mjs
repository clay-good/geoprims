// Settings that change how pages behave (web/app-shell, "Settings"): reduce
// motion before first paint, and the view every map opens in.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { serveBuiltSite } from './site.mjs';

test('reduce motion and the default map view apply on the next page', { timeout: 120_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const page = await (await browser.newContext({ serviceWorkers: 'block' })).newPage();
  await page.goto(`${origin}/settings/`);
  await page.selectOption('select[name=motion]', 'reduce');
  await page.selectOption('select[name=canvas]', 'globe');
  await page.goto(`${origin}/geodesy/magnetic/declination/`);
  assert.equal(await page.getAttribute('html', 'data-motion'), 'reduce', 'set before first paint');
  assert.equal(await page.$eval('.card', (e) => getComputedStyle(e).transitionDuration.split(',').every((d) => parseFloat(d) === 0)), true, 'no transitions');
  await page.waitForSelector('figure.map canvas');
  assert.equal(await page.$eval('.segmented [aria-pressed="true"]', (b) => b.textContent), 'Globe', 'the map opens as a globe');
  await page.goto(`${origin}/settings/`);
  await page.selectOption('select[name=motion]', 'system');
  await page.goto(`${origin}/`);
  assert.equal(await page.getAttribute('html', 'data-motion'), null);
});
