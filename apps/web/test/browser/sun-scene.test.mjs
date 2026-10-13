import assert from 'node:assert/strict';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { serveBuiltSite } from './site.mjs';

test('a sun permalink restores the 18:40 UTC terminator paused on the globe', { timeout: 120_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const context = await browser.newContext({ serviceWorkers: 'block', reducedMotion: 'reduce' });
  const page = await context.newPage();
  await page.goto(`${origin}/time/sun/position/`);

  const timeline = page.locator('.timeline');
  await timeline.waitFor();
  assert.equal(await timeline.getByRole('button').getAttribute('aria-pressed'), 'false');
  const range = timeline.getByLabel('Scene time');
  assert.equal(Number(await range.getAttribute('max')), 86_400);
  assert.equal(Number(await range.inputValue()), 64_800, 'the scene opens at the entered 18:00 UTC instant');

  await page.getByRole('button', { name: 'Globe' }).click();
  const canvas = page.locator('.map canvas[role="img"]');
  await canvas.waitFor();
  await page.waitForFunction(() => document.querySelector('.map canvas[role="img"]')?.getAttribute('aria-label')?.includes('2026-06-21T18:00:00Z'));

  await range.evaluate((input) => {
    input.value = '67200';
    input.dispatchEvent(new Event('input', { bubbles: true }));
  });
  await page.waitForFunction(() => document.querySelector('.map canvas[role="img"]')?.getAttribute('aria-label')?.includes('2026-06-21T18:40:00Z'));
  await page.waitForFunction(() => location.hash.startsWith('#v1:'));
  assert.match(await page.locator('.map').textContent(), /Day-night boundary · 2026-06-21T18:40:00Z/);

  const permalink = page.url();
  await page.reload();
  await page.waitForFunction(() => document.querySelector('.map canvas[role="img"]')?.getAttribute('aria-label')?.includes('2026-06-21T18:40:00Z'));
  assert.equal(page.url(), permalink);
  assert.ok(Math.abs(Number(await page.locator('.timeline input[type="range"]').inputValue()) - 67_200) < 0.11);
  assert.equal(await page.locator('.timeline button').getAttribute('aria-pressed'), 'false');
  assert.equal(await page.getByRole('button', { name: 'Globe' }).getAttribute('aria-pressed'), 'true');
});
