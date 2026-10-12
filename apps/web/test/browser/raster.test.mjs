import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { serveBuiltSite } from './site.mjs';

const axeSource = readFileSync(createRequire(import.meta.url).resolve('axe-core/axe.min.js'), 'utf8');

function tiledTiff(sample, scale = 10) {
  const header = [];
  const u16 = (value) => header.push(value & 255, value >>> 8);
  const u32 = (value) => header.push(value & 255, value >>> 8 & 255, value >>> 16 & 255, value >>> 24);
  const f64 = (value) => {
    const bytes = Buffer.alloc(8);
    bytes.writeDoubleLE(value);
    header.push(...bytes);
  };
  const entry = (tag, type, count, value) => { u16(tag); u16(type); u32(count); u32(value); };
  header.push(0x49, 0x49); u16(42); u32(8); u16(13);
  entry(256, 4, 1, 32);
  entry(257, 4, 1, 16);
  entry(258, 3, 1, 8);
  entry(259, 3, 1, 1);
  entry(262, 3, 1, 1);
  entry(277, 3, 1, 1);
  entry(284, 3, 1, 1);
  entry(322, 4, 1, 16);
  entry(323, 4, 1, 16);
  entry(324, 4, 2, 170);
  entry(325, 4, 2, 178);
  entry(33550, 12, 3, 186);
  entry(33922, 12, 6, 210);
  u32(0);
  u32(100_000); u32(200_000);
  u32(256); u32(256);
  f64(scale); f64(10); f64(0);
  for (const value of [0, 0, 0, 500_000, 4_500_000, 0]) f64(value);
  assert.equal(header.length, 258);

  const bytes = Buffer.alloc(200_256);
  Buffer.from(header).copy(bytes);
  for (let tile = 0; tile < 2; tile++) {
    const offset = tile ? 200_000 : 100_000;
    for (let i = 0; i < 256; i++) {
      const value = Math.floor(i / 16) * 32 + tile * 16 + i % 16;
      bytes[offset + i] = sample(value) & 255;
    }
  }
  return bytes;
}

test('a user can process local GeoTIFF bands into a preview and exact statistics', { timeout: 120_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const page = await browser.newPage();
  await page.goto(`${origin}/raster/index/ndvi/`);

  await page.getByText('Process local GeoTIFF files', { exact: true }).click();
  const input = page.locator('.raster-panel input[type="file"]');
  await input.setInputFiles([
    { name: 'nir.tif', mimeType: 'image/tiff', buffer: tiledTiff((value) => value) },
    { name: 'red.tif', mimeType: 'image/tiff', buffer: tiledTiff(() => 16) },
  ]);
  await page.locator('.raster-files strong', { hasText: 'nir.tif' }).waitFor();
  assert.match(await page.locator('.raster-files').innerText(), /32 × 16 px, 1 band/);

  await page.getByRole('button', { name: 'Process raster' }).click();
  await page.waitForFunction(() => document.querySelector('.raster-preview canvas, .raster-panel .bad'));
  assert.equal(await page.locator('.raster-panel .bad').count(), 0, (await page.locator('.raster-panel .bad').allTextContents()).join('; '));
  assert.equal(await page.locator('.raster-summary dd').allTextContents().then((values) => values[1]), '512');
  assert.equal(await page.locator('.raster-summary dd').allTextContents().then((values) => values[2]), '0');
  assert.equal(await page.locator('.raster-summary dd').first().innerText(), '0.6421313');
  assert.ok(await page.locator('.raster-preview canvas').evaluate((canvas) => {
    const pixels = canvas.getContext('2d').getImageData(0, 0, canvas.width, canvas.height).data;
    return pixels.some((channel, index) => index % 4 !== 3 && channel !== 181);
  }), 'preview contains computed colors rather than only no-data gray');
  assert.equal(await page.locator('.histogram-bars span').count(), 32);
  assert.match(await page.locator('.raster-histogram figcaption').innerText(), /512 preview pixels/);
  assert.equal(await page.locator('.raster-histogram > p.sr-only').innerText(), 'Preview histogram values');

  await page.locator('.raster-preview canvas').hover({ position: { x: 20, y: 20 } });
  assert.match(await page.locator('.cursor-readout').innerText(), / at .+, .+$/);

  await page.evaluate(axeSource);
  const violations = await page.evaluate(async () => (await window.axe.run(document, {
    runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'] },
    resultTypes: ['violations'],
  })).violations.map(({ id, help, nodes }) => ({ id, help, targets: nodes.map((node) => node.target) })));
  assert.deepEqual(violations, []);

  await page.goto(`${origin}/raster/index/ndvi/`);
  await page.getByText('Process local GeoTIFF files', { exact: true }).click();
  await page.locator('.raster-panel input[type="file"]').setInputFiles([
    { name: 'fine-nir.tif', mimeType: 'image/tiff', buffer: tiledTiff(() => 80, 10) },
    { name: 'coarse-red.tif', mimeType: 'image/tiff', buffer: tiledTiff(() => 20, 20) },
  ]);
  await page.getByRole('button', { name: 'Process raster' }).click();
  await page.getByText(/input raster grids differ/).waitFor();
  await page.locator('.raster-resample select').first().selectOption('bilinear');
  await page.locator('.raster-resample select').last().selectOption('right');
  await page.getByRole('button', { name: 'Resample and process' }).click();
  await page.locator('.raster-preview canvas').waitFor();
  assert.match(await page.getByText(/Processed with bilinear resampling/).innerText(), /Red reflectance grid/);

  await page.goto(`${origin}/raster/index/savi/`);
  await page.locator('#field-soil_factor').fill('0.25');
  await page.getByText('Process local GeoTIFF files', { exact: true }).click();
  await page.locator('.raster-panel input[type="file"]').setInputFiles([
    { name: 'nir.tif', mimeType: 'image/tiff', buffer: tiledTiff(() => 45) },
    { name: 'red.tif', mimeType: 'image/tiff', buffer: tiledTiff(() => 8) },
  ]);
  await page.getByRole('button', { name: 'Process raster' }).click();
  await page.locator('.raster-preview canvas').waitFor();
  assert.equal(await page.locator('.raster-panel .bad').count(), 0);
  assert.equal(await page.locator('.raster-summary dd').first().innerText(), '0.8685446');

  await page.goto(`${origin}/raster/index/evi/`);
  await page.getByText('Process local GeoTIFF files', { exact: true }).click();
  await page.locator('.raster-panel input[type="file"]').setInputFiles([
    { name: 'nir.tif', mimeType: 'image/tiff', buffer: tiledTiff(() => 45) },
    { name: 'red.tif', mimeType: 'image/tiff', buffer: tiledTiff(() => 8) },
    { name: 'blue.tif', mimeType: 'image/tiff', buffer: tiledTiff(() => 4) },
  ]);
  await page.getByRole('button', { name: 'Process raster' }).click();
  await page.locator('.raster-preview canvas').waitFor();
  assert.equal(await page.locator('.raster-panel .bad').count(), 0);
  assert.equal(await page.locator('.raster-summary dd').first().innerText(), '1.445313');

  await page.goto(`${origin}/raster/index/band-math/`);
  await page.getByText('Process local GeoTIFF files', { exact: true }).click();
  await page.locator('.raster-panel input[type="file"]').setInputFiles([
    { name: 'nir.tif', mimeType: 'image/tiff', buffer: tiledTiff(() => 45) },
    { name: 'red.tif', mimeType: 'image/tiff', buffer: tiledTiff(() => 8) },
  ]);
  await page.getByRole('button', { name: 'Process raster' }).click();
  await page.locator('.raster-preview canvas').waitFor();
  assert.equal(await page.locator('.raster-panel .bad').count(), 0);
  assert.equal(await page.locator('.raster-summary dd').first().innerText(), '0.6981132');
});
