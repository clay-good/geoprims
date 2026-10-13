import assert from 'node:assert/strict';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { serveBuiltSite } from './site.mjs';

async function cameraFrames(browser, origin, reducedMotion) {
  const context = await browser.newContext({ serviceWorkers: 'block', reducedMotion });
  const page = await context.newPage();
  await page.addInitScript(() => {
    const raf = window.requestAnimationFrame.bind(window);
    window.__cameraFrames = [];
    window.requestAnimationFrame = (callback) => raf((now) => {
      window.__cameraFrames.push(now);
      callback(now);
    });
  });
  await page.goto(`${origin}/navigation/geodesic/inverse/`);
  await page.waitForSelector('.map canvas[role="img"]');
  await page.waitForTimeout(700);
  await page.evaluate(() => { window.__cameraFrames = []; });
  await page.locator('#field-lat2').fill('0');
  await page.waitForTimeout(1_000);
  const frames = await page.evaluate(() => window.__cameraFrames);
  await context.close();
  return frames;
}

test('camera updates ease within 800 ms, and reduced motion makes them instant', { timeout: 120_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());

  const animated = await cameraFrames(browser, origin, 'no-preference');
  assert.ok(animated.length > 1, `the camera eased over ${animated.length} frames`);
  assert.ok(animated.at(-1) - animated[0] <= 800, 'camera easing stays within the 800 ms contract');

  const reduced = await cameraFrames(browser, origin, 'reduce');
  assert.equal(reduced.length, 0, 'reduced motion applies the new camera frame without animation');
});
