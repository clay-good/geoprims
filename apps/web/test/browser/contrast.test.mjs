// Contrast on rendered pages (web/visual-theme, "WCAG 2.2 AA conformance",
// "Contrast audit"): every visible piece of text, in both modes, measured
// against the background it is actually drawn on — ancestors' fills blended
// in order, and for text over the map, the map's own pixels underneath.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { serveBuiltSite } from './site.mjs';

const PAGES = [
  '/',
  '/tools/',
  '/aviation/',
  '/aviation/airspeed/',
  '/aviation/altimetry/density-altitude/',
  '/navigation/geodesic/inverse/',
  '/geodesy/magnetic/declination/',
  '/indexing/h3/grid-disk/',
  '/survey/curves/vertical-curve/',
  '/journeys/drone-mapping-day/',
  '/sources/',
];

/** Runs in the page: every text element's contrast against what is under it. */
function audit() {
  const parse = (c) => {
    const m = /rgba?\(([^)]+)\)/.exec(c);
    if (!m) return null;
    const p = m[1].split(/[ ,/]+/).filter(Boolean).map(Number);
    return [p[0], p[1], p[2], p.length > 3 ? p[3] : 1];
  };
  const over = (top, under) => [0, 1, 2].map((i) => top[i] * top[3] + under[i] * (1 - top[3])).concat(1);
  const lum = ([r, g, b]) => {
    const f = (v) => ((v /= 255) <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4);
    return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
  };
  const ratio = (a, b) => {
    const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p);
    return (x + 0.05) / (y + 0.05);
  };
  const canvasUnder = (el) => {
    const stage = el.closest('.map-stage, figure.map');
    const canvas = stage?.querySelector('canvas');
    if (!canvas) return null;
    const r = el.getBoundingClientRect();
    const c = canvas.getBoundingClientRect();
    if (r.right < c.left || r.left > c.right || r.bottom < c.top || r.top > c.bottom) return null;
    const k = canvas.width / c.width;
    const x = Math.max(0, Math.round((r.left + r.width / 2 - c.left) * k));
    const y = Math.max(0, Math.round((r.top + r.height / 2 - c.top) * k));
    const d = canvas.getContext('2d').getImageData(Math.min(x, canvas.width - 1), Math.min(y, canvas.height - 1), 1, 1).data;
    return [d[0], d[1], d[2], 1];
  };
  const background = (el) => {
    const layers = [];
    for (let n = el; n; n = n.parentElement) {
      const bg = parse(getComputedStyle(n).backgroundColor);
      if (bg && bg[3] > 0) layers.push(bg);
      if (bg && bg[3] >= 1) break;
      if (n.matches?.('.map-stage > *')) {
        const under = canvasUnder(n);
        if (under) { layers.push(under); break; }
      }
    }
    let base = parse(getComputedStyle(document.body).backgroundColor) ?? [255, 255, 255, 1];
    for (const l of layers.reverse()) base = over(l, base);
    return base;
  };
  const out = [];
  let checked = 0;
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  const seen = new Set();
  for (let t = walker.nextNode(); t; t = walker.nextNode()) {
    const el = t.parentElement;
    if (!el || seen.has(el) || !t.textContent.trim()) continue;
    seen.add(el);
    const s = getComputedStyle(el);
    const r = el.getBoundingClientRect();
    if (!r.width || !r.height || s.visibility === 'hidden' || el.closest('[hidden], .sr-only, [aria-hidden="true"] svg, svg, noscript, script, style, template, dialog:not([open]), [popover]:not(:popover-open), details:not([open]) > :not(summary)')) continue;
    if (el.closest('button:disabled, input:disabled, select:disabled')) continue; // WCAG exempts inactive controls
    let opacity = 1;
    for (let n = el; n; n = n.parentElement) opacity *= Number(getComputedStyle(n).opacity);
    if (opacity < 0.05) continue;
    const bg = background(el);
    const fg0 = parse(s.color);
    const fg = over([fg0[0], fg0[1], fg0[2], fg0[3] * opacity], bg);
    const size = parseFloat(s.fontSize);
    const large = size >= 24 || (size >= 18.66 && Number(s.fontWeight) >= 700);
    const need = large ? 3 : 4.5;
    const c = ratio(fg, bg);
    checked += 1;
    if (c < need) out.push({ text: t.textContent.trim().slice(0, 40), cls: `${el.tagName.toLowerCase()}.${el.className?.toString().split(' ')[0] ?? ''}`, ratio: Number(c.toFixed(2)), need });
  }
  return { checked, failures: out };
}

test('every visible text meets AA against its rendered background, in both modes', { timeout: 300_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const failures = [];
  let counted = 0;
  for (const mode of ['light', 'dark']) {
    const context = await browser.newContext({ serviceWorkers: 'block', colorScheme: mode, reducedMotion: 'reduce' });
    await context.addInitScript((m) => localStorage.setItem('gp-theme', m === 'dark' ? 'ink' : 'paper'), mode);
    const page = await context.newPage();
    for (const path of PAGES) {
      await page.goto(origin + path);
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(400);
      const { checked, failures: found } = await page.evaluate(audit);
      assert.ok(checked >= 15, `${mode} ${path}: only ${checked} text elements measured`);
      counted += checked;
      for (const f of found) failures.push({ mode, path, ...f });
    }
    await context.close();
  }
  t.diagnostic(`${counted} text elements measured`);
  if (failures.length) t.diagnostic(JSON.stringify(failures.slice(0, 40), null, 1));
  assert.equal(failures.length, 0, `${failures.length} text elements below AA`);
});

test('the audit catches low contrast, including text over the map', { timeout: 120_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const page = await (await browser.newContext({ serviceWorkers: 'block' })).newPage();
  await page.goto(`${origin}/geodesy/magnetic/declination/`);
  await page.waitForSelector('figure.map canvas');
  await page.evaluate(() => {
    const p = document.createElement('p');
    p.textContent = 'Planted pale text';
    p.style.color = 'rgb(200, 200, 200)';
    document.querySelector('main').append(p);
    // Surface-colored text on a see-through chip over the map fails against the map beneath.
    const chip = document.createElement('span');
    chip.textContent = 'Planted map label';
    chip.style.cssText = 'position:absolute;inset-block-start:40%;inset-inline-start:40%;color:var(--bg);background:transparent';
    document.querySelector('.map-stage').append(chip);
  });
  const { failures } = await page.evaluate(audit);
  const texts = failures.map((f) => f.text);
  assert.ok(texts.includes('Planted pale text'), 'pale text on the page is caught');
  assert.ok(texts.includes('Planted map label'), 'pale text over the map is caught');
});
