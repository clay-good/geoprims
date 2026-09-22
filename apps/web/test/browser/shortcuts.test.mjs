// Global shortcuts end to end (web/command-palette, "Global shortcuts"; 4.4):
// each key pressed on a real page does what the shortcut sheet says.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { serveBuiltSite } from './site.mjs';

test('each global shortcut acts on a real page', { timeout: 180_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const context = await browser.newContext({ serviceWorkers: 'block', permissions: ['clipboard-read', 'clipboard-write'] });
  const page = await context.newPage();
  const toast = () => page.$eval('.toast', (e) => e.textContent).catch(() => '');
  const press = async (k) => {
    await page.locator('body').click({ position: { x: 5, y: 5 } }).catch(() => {});
    await page.keyboard.press(k);
    await page.waitForTimeout(400);
  };

  await page.goto(`${origin}/navigation/geodesic/inverse/`);
  await page.waitForSelector('figure.map canvas');
  await page.waitForTimeout(800);
  const before = await page.inputValue('#field-lat1');
  const beforeB = await page.inputValue('#field-lat2');

  await press('s');
  assert.equal(await page.inputValue('#field-lat1'), beforeB, 's swaps A and B');
  assert.equal(await page.inputValue('#field-lat2'), before);
  assert.match(await toast(), /Swapped A and B/);

  const mode = () => page.$eval('.segmented [aria-pressed="true"]', (b) => b.textContent);
  const m0 = await mode();
  await press('c');
  assert.notEqual(await mode(), m0, 'c switches flat and globe');

  await page.waitForTimeout(600);
  await press('l');
  assert.match(await toast(), /Link copied/);
  assert.match(await page.evaluate(() => navigator.clipboard.readText()), /\/navigation\/geodesic\/inverse\/#v1:/, 'l copies the permalink');

  await press('y');
  assert.match(await toast(), /copied as JSON/);
  assert.ok(JSON.parse(await page.evaluate(() => navigator.clipboard.readText())).ok, 'y copies the result JSON');

  await press('p');
  assert.match(await toast(), /no scene to play/, 'p explains when there is nothing to play');

  await press('u');
  assert.match(await toast(), /^Units: /, 'u cycles the unit profile');

  await press(']');
  await page.waitForURL(/\/navigation\/geodesic\/(?!inverse\/)[^/]+\/$/);
  const next = new URL(page.url()).pathname;
  await press('[');
  await page.waitForURL(/\/navigation\/geodesic\/[^/]+\/$/);
  assert.notEqual(new URL(page.url()).pathname, next, '[ and ] step through the group');

  // A scene: p plays and pauses the closest-point-of-approach tool.
  await page.goto(`${origin}/navigation/route/cpa/`);
  await page.waitForSelector('.timeline');
  await press('p');
  assert.equal(await page.$eval('.timeline button', (b) => b.getAttribute('aria-pressed')), 'true', 'p plays');
  await press('p');
  assert.equal(await page.$eval('.timeline button', (b) => b.getAttribute('aria-pressed')), 'false', 'p pauses');

  // Never while typing.
  await page.goto(`${origin}/navigation/geodesic/inverse/`);
  await page.click('#field-lat1');
  const v = await page.inputValue('#field-lat1');
  await page.keyboard.press('s');
  assert.equal(await page.inputValue('#field-lat1'), `${v}s`, 'typing an s types an s');
});
