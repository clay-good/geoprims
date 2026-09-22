// Printing a tool page (web/visual-theme, "Print"; ux/mobile-and-field, the
// one-page calculation sheet): from the night mode, the printed page uses the
// paper palette and carries the inputs, the result, the formulas, the
// references, and a snapshot of the map, and none of the controls. The PDF
// itself is rendered and its pages counted.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { serveBuiltSite } from './site.mjs';

const visible = (page, sel) => page.$eval(sel, (el) => {
  const s = getComputedStyle(el);
  return s.display !== 'none' && s.visibility !== 'hidden' && el.getClientRects().length > 0;
}).catch(() => false);

test('a tool page prints as a paper calculation sheet with its formula, sources, and map', { timeout: 120_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const context = await browser.newContext({ serviceWorkers: 'block', colorScheme: 'dark' });
  const page = await context.newPage();
  await page.goto(`${origin}/navigation/geodesic/inverse/`);
  await page.evaluate(() => document.documentElement.setAttribute('data-theme', 'dark'));
  await page.waitForSelector('figure.map canvas');
  await page.waitForTimeout(1200); // the map eases into view
  await page.evaluate(() => dispatchEvent(new Event('beforeprint')));
  await page.emulateMedia({ media: 'print' });

  const bg = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
  assert.equal(bg, 'rgb(255, 255, 255)', 'paper, not night');
  assert.ok(await page.$eval('details.proof', (d) => d.open), 'the proof panel opens for printing');
  for (const sel of ['.card.inputs', '.card.answer', '.trace-formula', 'details.proof ul.sources li', 'figure.map canvas']) {
    assert.ok(await visible(page, sel), `${sel} is on the sheet`);
  }
  for (const sel of ['.trace-sub', '.worked', '.map-bar', 'header.site', '.actions']) {
    assert.ok(!(await visible(page, sel)), `${sel} is printed`);
  }
  // The map is a real snapshot: more than one color on the canvas.
  const colors = await page.$eval('figure.map canvas', (c) => {
    const d = c.getContext('2d').getImageData(0, 0, c.width, c.height).data;
    const seen = new Set();
    for (let i = 0; i < d.length; i += 400) seen.add(`${d[i] >> 4},${d[i + 1] >> 4},${d[i + 2] >> 4}`);
    return seen.size;
  });
  assert.ok(colors > 3, `the map drew (${colors} colors)`);

  // Afterwards the panel goes back as it was.
  await page.evaluate(() => dispatchEvent(new Event('afterprint')));
  assert.equal(await page.$eval('details.proof', (d) => d.open), false);

  // A real print: page.pdf() fires beforeprint and afterprint itself.
  const pdf = await page.pdf({ format: 'Letter', printBackground: true });
  const pages = (pdf.toString('latin1').match(/\/Type\s*\/Page[^s]/g) ?? []).length;
  assert.ok(pdf.length > 20_000, 'a real PDF');
  assert.ok(pages >= 1 && pages <= 2, `${pages} pages`);
  t.diagnostic(`geodesic inverse sheet: ${pages} page(s), ${Math.round(pdf.length / 1024)} KB`);
  assert.equal(await page.$eval('details.proof', (d) => d.open), false, 'closed again after the print');
});

test('a single-result tool without a map prints on one page', { timeout: 120_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const page = await (await browser.newContext({ serviceWorkers: 'block' })).newPage();
  await page.goto(`${origin}/aviation/altimetry/density-altitude/`);
  await page.waitForSelector('.card.answer');
  await page.evaluate(() => dispatchEvent(new Event('beforeprint')));
  await page.emulateMedia({ media: 'print' });
  const pdf = await page.pdf({ format: 'Letter' });
  const pages = (pdf.toString('latin1').match(/\/Type\s*\/Page[^s]/g) ?? []).length;
  t.diagnostic(`density altitude sheet: ${pages} page(s)`);
  assert.equal(pages, 1, `the calculation sheet runs to ${pages} pages`);
});
