import assert from 'node:assert/strict';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { serveBuiltSite } from './site.mjs';

test('the geographic contour example draws contour lines on the map', { timeout: 120_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const context = await browser.newContext({ serviceWorkers: 'block', reducedMotion: 'reduce' });
  const page = await context.newPage();
  await page.goto(`${origin}/raster/terrain/contours/`);

  const canvas = page.locator('.map canvas[role="img"]');
  await canvas.waitFor();
  await page.waitForFunction(() => document.querySelector('.map canvas[role="img"]')?.getAttribute('aria-label')?.includes('contour line'));
  assert.match(await canvas.getAttribute('aria-label'), /contour line/);
  assert.equal(await page.locator('.map-legend').getByText('Contours: equal elevation').count(), 1);
  assert.equal(await page.locator('#field-cell_degrees').inputValue(), '0.0001');
  assert.equal(await page.locator('#field-north_west_lat').inputValue(), '40');
  assert.equal(await page.locator('#field-north_west_lon').inputValue(), '-105');
  const box = await canvas.boundingBox();
  assert.ok(box && box.width > 200 && box.height > 100, 'the contour map is visible');
});
