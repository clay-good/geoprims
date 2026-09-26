// Mobile quality gates (add-glanceable-and-field-ux 3.5, ux/mobile-and-field
// "Mobile quality gates"): every built page at 320 × 720 in Chromium and
// WebKit, and a sample of pages at 568 × 320 landscape and at 375 px with 200%
// text zoom, must not scroll sideways. Each page must answer 2xx first, so an
// error page cannot pass, and a failure names the route and the element that
// sticks out.
import assert from 'node:assert/strict';
import { readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { serveBuiltSite, web } from './site.mjs';

const dist = join(web, 'dist');

/** Every built page as its route ("dist/a/index.html" is "/a/"). */
function routes(dir = dist, out = []) {
  for (const f of readdirSync(dir)) {
    const p = join(dir, f);
    if (statSync(p).isDirectory()) routes(p, out);
    else if (f === 'index.html') out.push(p.slice(dist.length).replace(/index\.html$/, '') || '/');
  }
  return out.sort();
}

/** In the page: null if nothing sticks out, else the page width and the outermost element past the edge. */
function overflow() {
  const d = document.documentElement;
  if (d.scrollWidth <= d.clientWidth + 1) return null;
  const edge = d.clientWidth + 1;
  const name = (e) => `${e.tagName.toLowerCase()}${e.id ? `#${e.id}` : ''}${e.classList.length ? `.${[...e.classList].join('.')}` : ''}`;
  // The outermost element that reaches past the edge while its parent does not.
  // Something inside a scrolling box is clipped by it, so it cannot widen the page.
  const clipped = (e) => {
    for (let a = e.parentElement; a && a !== document.body; a = a.parentElement) if (getComputedStyle(a).overflowX !== 'visible') return true;
    return false;
  };
  for (const e of document.body.querySelectorAll('*')) {
    const r = e.getBoundingClientRect();
    if (r.width === 0 || r.right <= edge || clipped(e)) continue;
    const p = e.parentElement?.getBoundingClientRect();
    if (!p || p.right <= edge) return `${d.scrollWidth} > ${d.clientWidth}: ${name(e)}`;
  }
  return `${d.scrollWidth} > ${d.clientWidth}`;
}

async function sweep(browser, origin, paths, viewport, { zoom = false } = {}) {
  const context = await browser.newContext({ viewport });
  const page = await context.newPage();
  const problems = [];
  for (const path of paths) {
    const res = await page.goto(origin + path, { waitUntil: 'load' });
    if (!res?.ok()) {
      problems.push(`${path} answered ${res?.status()}`);
      continue;
    }
    // Text zoom: the reader's text setting, not a page zoom, so the layout width stays 375 px.
    // Set through the CSSOM: the site's CSP rightly refuses an injected style tag.
    if (zoom) await page.evaluate(() => document.documentElement.style.setProperty('font-size', '200%', 'important'));
    await page.waitForTimeout(60);
    const over = await page.evaluate(overflow);
    if (over) problems.push(`${path} at ${viewport.width} × ${viewport.height}${zoom ? ' with 200% text' : ''} scrolls sideways: ${over}`);
  }
  await context.close();
  return problems;
}

/** Every hub, the home page, and at least 30 tools spread across domains. */
function sample(all) {
  const hubs = all.filter((r) => r.split('/').length <= 4 && !r.includes('/learn/') && !r.includes('/workflows/'));
  const tools = all.filter((r) => r.split('/').length === 5);
  const byDomain = new Map();
  for (const r of tools) {
    const d = r.split('/')[1];
    if (!byDomain.has(d)) byDomain.set(d, []);
    byDomain.get(d).push(r);
  }
  // Round-robin across domains, so no one domain fills the sample.
  const picked = [];
  for (let i = 0; picked.length < 40; i++) {
    let any = false;
    for (const list of byDomain.values()) if (list[i * 7]) (picked.push(list[i * 7]), (any = true));
    if (!any) break;
  }
  return [...new Set(['/', ...hubs, ...picked])];
}

for (const [name, engine] of [['Chromium', chromium], ['WebKit', webkit]]) {
  test(`every page reflows at 320 × 720 in ${name}`, { timeout: 900_000 }, async (t) => {
    const all = routes();
    assert.ok(all.length > 450, `${all.length} pages`);
    const origin = await serveBuiltSite(t);
    const browser = await engine.launch({ headless: true });
    t.after(() => browser.close());
    const problems = await sweep(browser, origin, all, { width: 320, height: 720 });
    assert.deepEqual(problems, [], problems.join('\n'));
  });

  test(`sampled pages reflow in landscape and with 200% text in ${name}`, { timeout: 600_000 }, async (t) => {
    const paths = sample(routes());
    assert.ok(paths.filter((p) => p.split('/').length === 5).length >= 30, 'at least 30 tools');
    const origin = await serveBuiltSite(t);
    const browser = await engine.launch({ headless: true });
    t.after(() => browser.close());
    const problems = [
      ...(await sweep(browser, origin, paths, { width: 568, height: 320 })),
      ...(await sweep(browser, origin, paths, { width: 375, height: 812 }, { zoom: true })),
    ];
    assert.deepEqual(problems, [], problems.join('\n'));
  });
}
