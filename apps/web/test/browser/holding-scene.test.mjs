import assert from 'node:assert/strict';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { serveBuiltSite } from './site.mjs';

test('the holding playhead scrubs, persists, and still works with reduced motion', { timeout: 120_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const context = await browser.newContext({ serviceWorkers: 'block', reducedMotion: 'reduce' });
  const page = await context.newPage();
  await page.goto(`${origin}/aviation/ifr/hold-wind-timing/`);
  const timeline = page.locator('.timeline');
  await timeline.waitFor();
  assert.equal(await timeline.getByRole('button').getAttribute('aria-pressed'), 'false');
  const range = timeline.getByLabel('Scene time');
  const end = Number(await range.getAttribute('max'));
  assert.ok(Math.abs(Number(await range.inputValue()) - end) < 1e-9);
  const aircraft = page.locator('.diagram polygon.dg-dot-now');
  await aircraft.waitFor();
  const initialPoints = await aircraft.getAttribute('points');

  const scrubTo = end / 4;
  await range.evaluate((input, value) => {
    input.value = String(value);
    input.dispatchEvent(new Event('input', { bubbles: true }));
  }, scrubTo);
  await page.waitForFunction((points) => document.querySelector('.diagram polygon.dg-dot-now')?.getAttribute('points') !== points, initialPoints);
  await page.waitForFunction(() => location.hash.startsWith('#v1:'));
  assert.match(await page.locator('.diagram').textContent(), /heading [\d.]+°/);
  const scrubbed = Number(await range.inputValue());

  const permalink = page.url();
  await page.reload();
  await page.waitForSelector('.diagram polygon.dg-dot-now');
  assert.equal(page.url(), permalink);
  assert.ok(Math.abs(Number(await page.locator('.timeline input[type="range"]').inputValue()) - scrubbed) < 0.11);
  assert.equal(await page.locator('.timeline button').getAttribute('aria-pressed'), 'false');
});
