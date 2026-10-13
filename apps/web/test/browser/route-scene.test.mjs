import assert from 'node:assert/strict';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { serveBuiltSite } from './site.mjs';

test('a route permalink restores its core playhead paused with reduced motion', { timeout: 120_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const context = await browser.newContext({ serviceWorkers: 'block', reducedMotion: 'reduce' });
  const page = await context.newPage();
  await page.goto(`${origin}/navigation/route/legs/`);

  const timeline = page.locator('.timeline');
  await timeline.waitFor();
  assert.equal(await timeline.getByRole('button').getAttribute('aria-pressed'), 'false');
  const range = timeline.getByLabel('Scene time');
  const end = Number(await range.getAttribute('max'));
  assert.ok(end > 0);
  assert.ok(Math.abs(Number(await range.inputValue()) - end) < 0.11, 'the route opens complete');

  const scrubbed = Math.round(end / 2);
  await range.evaluate((input, value) => {
    input.value = String(value);
    input.dispatchEvent(new Event('input', { bubbles: true }));
  }, scrubbed);
  await page.waitForFunction(() => document.querySelector('.map canvas[role="img"]')?.getAttribute('aria-label')?.includes('the planned route'));
  const description = await page.locator('.map canvas[role="img"]').getAttribute('aria-label');
  assert.match(description, /the route flown through the playhead/);
  assert.match(description, /the aircraft at the playhead/);
  await page.waitForFunction(() => location.hash.startsWith('#v1:'));
  const saved = Number(await range.inputValue());
  assert.ok(Math.abs(saved - scrubbed) <= 60, 'the range step stays within one minute of the requested midpoint');

  const permalink = page.url();
  await page.reload();
  await page.waitForFunction(() => document.querySelector('.map canvas[role="img"]')?.getAttribute('aria-label')?.includes('the planned route'));
  assert.equal(page.url(), permalink);
  assert.ok(Math.abs(Number(await page.locator('.timeline input[type="range"]').inputValue()) - saved) < 0.11);
  assert.equal(await page.locator('.timeline button').getAttribute('aria-pressed'), 'false');
});
