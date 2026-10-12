// Natural Earth base map (web/map-canvas): zooming swaps one whole 50m file,
// and map or globe interaction never asks for location-revealing tiles.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
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

  const canvas = page.locator('.map-stage canvas:not(.map-gpu)');
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

test('a 2D map downloads as a standalone vector SVG', { timeout: 120_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const page = await browser.newPage();
  await page.goto(`${origin}/navigation/geodesic/inverse/`);
  await page.waitForFunction(() => document.querySelector('.map-readout')?.textContent.includes('Natural Earth 110m'));
  await page.getByRole('button', { name: 'Map', exact: true }).click();
  const pending = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Download the map as SVG, with attribution' }).click();
  const download = await pending;
  assert.equal(download.suggestedFilename(), 'navigation.geodesic.inverse.svg');
  const svg = readFileSync(await download.path(), 'utf8');
  assert.match(svg, /^<svg[^>]*viewBox=/);
  assert.match(svg, /data-layer="land"/);
  assert.match(svg, /data-layer="line"/);
  assert.match(svg, /Base map: Natural Earth \(public domain\)/);
  assert.ok(!svg.includes('<image'), 'the download contains vectors, not a raster screenshot');
});

test('WebGPU and WebGL2 present the same pixels as the Canvas2D fallback', { timeout: 120_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  for (const [expected, args] of [
    ['webgpu', ['--enable-unsafe-webgpu', '--use-angle=swiftshader']],
    ['webgl', ['--disable-webgpu']],
  ]) {
    const browser = await chromium.launch({ headless: true, args });
    t.after(() => browser.close());
    const page = await browser.newPage();
    const warnings = [];
    page.on('console', (message) => warnings.push(message.text()));
    await page.goto(`${origin}/navigation/geodesic/inverse/`);
    await page.waitForFunction(() => {
      const canvas = document.querySelector('.map-stage canvas:not(.map-gpu)');
      return canvas?.dataset.renderer === 'webgl' || canvas?.dataset.renderer === 'webgpu' || document.querySelector('.map-capability');
    });
    const backend = await page.locator('.map-stage canvas:not(.map-gpu)').getAttribute('data-renderer');
    assert.equal(backend, expected, warnings.join(' | '));
    const stage = page.locator('.map-stage');
    const gpuShot = await stage.screenshot();
    await page.locator('.map-gpu').evaluate((canvas) => (canvas.hidden = true));
    const sourceShot = await stage.screenshot();

    const result = await page.evaluate(async ({ gpuPng, sourcePng }) => {
      async function pixels(base64) {
        const binary = atob(base64);
        const bytes = Uint8Array.from(binary, (byte) => byte.charCodeAt(0));
        const image = await createImageBitmap(new Blob([bytes], { type: 'image/png' }));
        const canvas = document.createElement('canvas');
        canvas.width = image.width;
        canvas.height = image.height;
        const g = canvas.getContext('2d');
        g.drawImage(image, 0, 0);
        return g.getImageData(0, 0, canvas.width, canvas.height).data;
      }
      const sourcePixels = await pixels(sourcePng);
      const gpuPixels = await pixels(gpuPng);
      let changed = 0;
      for (let i = 0; i < sourcePixels.length; i++) if (Math.abs(sourcePixels[i] - gpuPixels[i]) > 1) changed++;
      return { changed, channels: sourcePixels.length };
    }, {
      gpuPng: gpuShot.toString('base64'),
      sourcePng: sourceShot.toString('base64'),
    });
    assert.ok(result.changed / result.channels < 0.001, `${expected}: ${result.changed} of ${result.channels} channels differ`);
  }
});
