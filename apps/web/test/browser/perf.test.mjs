// Performance budgets (build-web-experience 1.4; contracts/reference-profiles):
// 50 sampled routes, each loaded cold three times under the reference profile
// (CPU throttled to the target BenchmarkIndex; 9 Mbps down, 1.5 Mbps up, 150 ms round trip; no cache) at
// the typical-phone viewport, with the median held to the hard budgets:
// LCP 2.0 s, interactive 2.5 s, INP 200 ms, CLS 0.1, and shell JavaScript
// 90 KB compressed. PERF_SAMPLE=n measures only the first n routes.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { gzipSync } from 'node:zlib';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { cpuRate } from '../../scripts/cpu.mjs';
import { serveBuiltSite, web } from './site.mjs';

const root = join(web, '../..');
const profile = JSON.parse(readFileSync(join(root, 'data/reference-profile.json'), 'utf8'));
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
export const BUDGET = { lcp: 2000, interactive: 2500, inp: 200, cls: 0.1, shellJsKb: 90 };
const phone = profile.viewports.find((v) => v.aboveTheFold);

/** 50 routes: the home page, the index, every domain, a few groups, and tools spread over the catalog. */
export function sampleRoutes(n = 50) {
  const route = (id) => `/${id.split('.').join('/')}/`;
  const domains = [...new Set(catalog.tools.map((t) => t.domain))];
  const groups = [...new Set(catalog.tools.map((t) => `${t.domain}.${t.group}`))];
  const fixed = ['/', '/tools/', ...domains.map(route), ...groups.filter((_, i) => i % 12 === 0).map(route)];
  const tools = catalog.tools.filter((t) => t.composedOf.length === 0);
  const left = n - fixed.length;
  const step = tools.length / left;
  return [...fixed, ...Array.from({ length: left }, (_, i) => route(tools[Math.floor(i * step)].id))];
}

const median = (xs) => [...xs].sort((a, b) => a - b)[Math.floor(xs.length / 2)];

async function measure(browser, origin, path) {
  const context = await browser.newContext({ serviceWorkers: 'block', viewport: { width: phone.width, height: phone.height } });
  const page = await context.newPage();
  const cdp = await context.newCDPSession(page);
  await cdp.send('Emulation.setCPUThrottlingRate', { rate: (await cpuRate(browser, profile)).rate });
  await cdp.send('Network.enable');
  await cdp.send('Network.setCacheDisabled', { cacheDisabled: true });
  await cdp.send('Network.emulateNetworkConditions', {
    offline: false,
    latency: profile.network.rttMs,
    downloadThroughput: (profile.network.downKbps * 1000) / 8,
    uploadThroughput: (profile.network.upKbps * 1000) / 8,
  });
  await page.addInitScript(() => {
    const p = (window.__perf = { lcp: 0, cls: 0, inp: 0 });
    new PerformanceObserver((l) => l.getEntries().forEach((e) => (p.lcp = e.startTime))).observe({ type: 'largest-contentful-paint', buffered: true });
    new PerformanceObserver((l) => l.getEntries().forEach((e) => !e.hadRecentInput && (p.cls += e.value))).observe({ type: 'layout-shift', buffered: true });
    new PerformanceObserver((l) => l.getEntries().forEach((e) => e.interactionId && (p.inp = Math.max(p.inp, e.duration)))).observe({ type: 'event', buffered: true, durationThreshold: 16 });
  });
  // Shell JavaScript: every script the page loads, compressed as a host would.
  const scripts = [];
  page.on('response', async (res) => {
    if (res.request().resourceType() === 'script' && res.url().startsWith(origin)) scripts.push(res.body().then((b) => gzipSync(b).length, () => 0));
  });
  await page.goto(origin + path, { waitUntil: 'load' });
  await page.waitForFunction(() => !document.querySelector('astro-island[ssr]') && (window.__perf.interactive ??= performance.now()), null, { timeout: 30_000 });
  await page.waitForTimeout(300);
  const lcp = await page.evaluate(() => window.__perf.lcp);
  // One interaction: type into the first field of a tool, or press the mode toggle elsewhere.
  // Visible only: a tool whose first field is a textarea keeps its plain text
  // inputs inside the collapsed "more options" disclosure, which cannot be clicked.
  const field = await page.$('.card.inputs textarea:visible, .card.inputs input[type="text"]:visible, .card.inputs input:not([type]):visible');
  if (field) {
    await field.click();
    await page.keyboard.type('1');
    await page.keyboard.press('Backspace');
  } else {
    const toggle = await page.$('header.site [data-theme-toggle], header.site button:last-of-type');
    if (toggle) { await toggle.click(); await toggle.click(); }
  }
  await page.waitForTimeout(500);
  const m = await page.evaluate(() => window.__perf);
  const shellJs = (await Promise.all(scripts)).reduce((a, b) => a + b, 0) / 1024;
  await context.close();
  return { lcp, interactive: m.interactive, inp: m.inp, cls: m.cls, shellJsKb: shellJs };
}

test(`performance budgets on sampled routes under reference profile ${profile.version}`, { timeout: 3_600_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const routes = sampleRoutes().slice(0, Number(process.env.PERF_SAMPLE) || undefined);
  const rows = [];
  for (const path of routes) {
    const runs = [];
    for (let i = 0; i < profile.measurement.runsPerRoute; i++) runs.push(await measure(browser, origin, path));
    const row = { path, ...Object.fromEntries(Object.keys(BUDGET).map((k) => [k, median(runs.map((r) => r[k]))])) };
    rows.push(row);
    t.diagnostic(`${path} lcp ${Math.round(row.lcp)} ms · interactive ${Math.round(row.interactive)} ms · inp ${Math.round(row.inp)} ms · cls ${row.cls.toFixed(3)} · js ${row.shellJsKb.toFixed(1)} KB`);
  }
  const over = rows.flatMap((r) => Object.entries(BUDGET).filter(([k, b]) => r[k] > b).map(([k, b]) => `${r.path}: ${k} ${Math.round(r[k] * 1000) / 1000} over ${b}`));
  const worst = Object.fromEntries(Object.keys(BUDGET).map((k) => [k, Math.max(...rows.map((r) => r[k]))]));
  t.diagnostic(`worst: ${JSON.stringify(worst)}`);
  assert.equal(routes.length, Number(process.env.PERF_SAMPLE) || 50);
  assert.deepEqual(over, []);
});
