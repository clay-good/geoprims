// Natural Earth base map (web/map-canvas): zooming swaps one whole 50m file,
// and map or globe interaction never asks for location-revealing tiles.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { serveBuiltSite } from './site.mjs';

test('zooming loads one detailed Natural Earth file and no map tiles', { timeout: 120_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const page = await browser.newPage();
  const requests = [];
  page.on('request', (request) => requests.push(new URL(request.url()).pathname));
  await page.goto(`${origin}/navigation/geodesic/inverse/`);
  await page.waitForFunction(() => document.querySelector('.map-readout')?.textContent.includes('Natural Earth 110m'));

  const zoom = page.getByRole('button', { name: 'Zoom in' });
  for (let i = 0; i < 18; i++) await zoom.click();
  await page.waitForFunction(() => document.querySelector('.map-readout')?.textContent.includes('Natural Earth 50m'));

  const canvas = page.locator('.map-stage canvas');
  const box = await canvas.boundingBox();
  await page.mouse.move(box.x + box.width * 0.4, box.y + box.height * 0.5);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width * 0.6, box.y + box.height * 0.55);
  await page.mouse.up();
  await page.getByRole('button', { name: 'Globe' }).click();
  await page.mouse.move(box.x + box.width * 0.45, box.y + box.height * 0.5);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width * 0.55, box.y + box.height * 0.5);
  await page.mouse.up();

  const detail = requests.filter((path) => path === '/assets/ne-50m/5.1.2/ne-50m.json');
  assert.equal(detail.length, 1, 'the detailed base map is one whole-file request');
  assert.deepEqual(requests.filter((path) => /\/tiles?\//i.test(path)), [], 'no basemap tile request');
  assert.deepEqual(requests.filter((path) => /naturalearthdata|mapbox|google|esri/i.test(path)), [], 'no map service request');
});
