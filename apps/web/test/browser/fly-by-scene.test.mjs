import assert from 'node:assert/strict';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { serveBuiltSite } from './site.mjs';

test('the fly-by playhead scrubs, persists, and still works with reduced motion', { timeout: 120_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const context = await browser.newContext({ serviceWorkers: 'block', reducedMotion: 'reduce' });
  const page = await context.newPage();
  await page.goto(`${origin}/navigation/route/fly-by/`);
  const timeline = page.locator('.timeline');
  await timeline.waitFor();
  assert.equal(await timeline.getByRole('button').getAttribute('aria-pressed'), 'false');
  const range = timeline.getByLabel('Scene time');
  const end = Number(await range.getAttribute('max'));
  assert.ok(Math.abs(Number(await range.inputValue()) - end) < 1e-9);
  await page.waitForSelector('.diagram polygon.dg-dot-now');
  assert.match(await page.locator('.diagram').textContent(), /course 90°/);

  const halfway = end / 2;
  await range.evaluate((input, value) => {
    input.value = String(value);
    input.dispatchEvent(new Event('input', { bubbles: true }));
  }, halfway);
  const scrubbed = Number(await range.inputValue());
  await page.waitForFunction(() => location.hash.startsWith('#v1:'));
  await page.waitForSelector('.diagram polygon.dg-dot-now');
  assert.match(await page.locator('.diagram').textContent(), /t = [\d.]+ s · course 45°/);

  const permalink = page.url();
  await page.reload();
  // The built page ships the example's own scene, so the aircraft and the
  // range are in the static HTML at the end of the leg, before any script
  // runs. Waiting for the dot alone reads that page, not the restored one:
  // the restored frame is the one that turns onto the new course.
  await page.waitForFunction(() => /t = [\d.]+ s · course 45°/.test(document.querySelector('.diagram')?.textContent ?? ''));
  assert.equal(page.url(), permalink);
  assert.ok(Math.abs(Number(await page.locator('.timeline input[type="range"]').inputValue()) - scrubbed) < 0.11);
  assert.equal(await page.locator('.timeline button').getAttribute('aria-pressed'), 'false');
});
