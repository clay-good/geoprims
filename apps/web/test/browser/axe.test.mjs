// axe-core in every mode (build-web-experience 2.3; web/visual-theme, "WCAG
// 2.2 AA conformance"): representative pages, the palette open, and a tool
// with its answer and map, checked against WCAG 2.2 A and AA in both modes.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { serveBuiltSite } from './site.mjs';

const axeSource = readFileSync(createRequire(import.meta.url).resolve('axe-core/axe.min.js'), 'utf8');
const PAGES = [
  '/',
  '/tools/',
  '/aviation/',
  '/aviation/airspeed/',
  '/aviation/altimetry/density-altitude/',
  '/navigation/geodesic/inverse/',
  '/indexing/h3/grid-disk/',
  '/survey/land/deed-plot/',
  '/journeys/vfr-preflight/',
  '/sources/',
  '/accuracy/',
  '/404.html',
];

async function check(page) {
  await page.evaluate(axeSource);
  return page.evaluate(async () => {
    const r = await window.axe.run(document, { runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'] }, resultTypes: ['violations'] });
    return r.violations.map((v) => ({ id: v.id, impact: v.impact, help: v.help, nodes: v.nodes.slice(0, 3).map((n) => n.target.join(' ')) }));
  });
}

test('axe finds no WCAG 2.2 A or AA violation on representative pages, in both modes', { timeout: 600_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const found = [];
  for (const mode of ['paper', 'ink']) {
    const context = await browser.newContext({ serviceWorkers: 'block', reducedMotion: 'reduce' });
    await context.addInitScript((m) => localStorage.setItem('gp-theme', m), mode);
    const page = await context.newPage();
    for (const path of PAGES) {
      await page.goto(origin + path);
      await page.waitForLoadState('networkidle');
      for (const v of await check(page)) found.push({ mode, path, ...v });
    }
    // The palette, open.
    await page.goto(`${origin}/aviation/`);
    await page.keyboard.press('/');
    await page.keyboard.type('density');
    await page.waitForTimeout(500);
    for (const v of await check(page)) found.push({ mode, path: 'palette', ...v });
    // The proof panel and the batch panel, open.
    await page.goto(`${origin}/aviation/altimetry/density-altitude/`);
    await page.waitForLoadState('networkidle');
    await page.evaluate(() => document.querySelectorAll('details').forEach((d) => (d.open = true)));
    await page.waitForTimeout(300);
    for (const v of await check(page)) found.push({ mode, path: 'panels open', ...v });
    await context.close();
  }
  if (found.length) t.diagnostic(JSON.stringify(found, null, 1).slice(0, 6000));
  assert.equal(found.length, 0, `${found.length} axe violations`);
});

test('the axe check is live: a planted image without alt text and a nameless button are reported', { timeout: 120_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const page = await (await browser.newContext({ serviceWorkers: 'block' })).newPage();
  await page.goto(`${origin}/sources/`);
  await page.evaluate(() => {
    const img = document.createElement('img');
    img.src = '/favicon.svg';
    const button = document.createElement('button');
    button.type = 'button';
    document.querySelector('main').append(img, button);
  });
  const ids = (await check(page)).map((v) => v.id);
  assert.ok(ids.includes('image-alt'), `image-alt not reported: ${ids}`);
  assert.ok(ids.includes('button-name'), `button-name not reported: ${ids}`);
});
