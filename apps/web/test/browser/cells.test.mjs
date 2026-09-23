// A million-cell answer on the map (add-spatial-indexing-and-raster 1.8): the
// largest fill the core will answer at all is 1,000,000 cells, and it never
// hands over a million outlines. Past its listing limit it compacts, so an
// 889,954-cell fill arrives as 6,256 cells covering the same ground, which is
// the level of detail that exists today. This draws that set over the base map
// with the real 2D renderer while the view pans, with the CPU throttled to the reference profile's target device.
//
// It does not yet hold a 60 Hz frame, and the number here says so rather than
// hiding it: p95 is about 90 ms, because 6,256 cells are 6,256 fills and
// strokes and each costs far more than its six vertices do. At this zoom a
// cell is 2.86 px across, so the covering reads as a solid area and the
// individual cells cannot be told apart anyway; drawing each cell's parent at
// a resolution chosen from the view would cut the count by roughly ten and
// bring it inside the frame. That needs the view before the layers are built,
// and today the view is framed from them, so it is a change to the shape of
// the drawing rather than a tuning. Measured alternatives, at 4x CPU: one path
// for every cell built through trace() costs 250-280 ms, worse than a path
// each, and a Path2D built once and reused costs 7.3 ms but cannot be reused
// while the view is moving. The ceiling below is there to catch a regression,
// not to bless the number.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { cpuRate } from '../../scripts/cpu.mjs';
import { nodeHost } from '../../../../packages/runtime/src/node.mjs';
import { web } from './site.mjs';

const root = join(web, '../..');
const profile = JSON.parse(readFileSync(join(root, 'data/reference-profile.json'), 'utf8'));
const FILES = { '/projection.js': 'src/lib/map/projection.js', '/render.js': 'src/lib/map/render.js', '/ne.json': 'public/basemap/ne-110m.json' };

/** The compacted stand-in for a fill near the core's one-million-cell ceiling. */
async function scene() {
  const host = nodeHost(join(root, 'dist/wasm'));
  const args = {
    points: [
      { lat: 40.0, lon: -80.35 },
      { lat: 40.0, lon: -78.85 },
      { lat: 40.95, lon: -78.85 },
      { lat: 40.95, lon: -80.35 },
    ],
    resolution: 10,
  };
  const fill = JSON.parse(await host.invoke('indexing.h3.polygon-to-cells', JSON.stringify(args)));
  assert.equal(fill.ok, true, JSON.stringify(fill.error));
  assert.ok(fill.result.count > 800_000, `${fill.result.count} cells`);
  assert.equal(fill.result.cells, undefined, 'too many to list, so compacted');
  const ids = fill.result.compacted.map((c) => c.cell ?? c);
  const info = JSON.parse(await host.invokeBatch('indexing.h3.cell-info', JSON.stringify(ids.map((cell) => ({ cell })))));
  const rings = info.map((r) => r.result.boundary.map((p) => [p.lon, p.lat]));
  return { count: fill.result.count, rings };
}

const CEILING = 140;

test('a million-cell answer draws its compacted stand-in, whole', { timeout: 300_000 }, async (t) => {
  const { count, rings } = await scene();
  t.diagnostic(`${count} cells compact to ${rings.length}, ${rings.reduce((a, r) => a + r.length, 0)} vertices`);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  await page.route('http://bench.local/**', (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path === '/') return route.fulfill({ contentType: 'text/html', body: '<!doctype html><canvas width="1280" height="800" style="width:1280px;height:800px"></canvas>' });
    if (path === '/cells.json') return route.fulfill({ contentType: 'application/json', body: JSON.stringify(rings) });
    const file = FILES[path];
    return file ? route.fulfill({ contentType: path.endsWith('.json') ? 'application/json' : 'text/javascript', body: readFileSync(join(web, file)) }) : route.fulfill({ status: 404 });
  });
  await page.goto('http://bench.local/');
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Emulation.setCPUThrottlingRate', { rate: (await cpuRate(browser, profile)).rate });
  const result = await page.evaluate(async () => {
    const { decode, frame } = await import('/projection.js');
    const { draw } = await import('/render.js');
    const ne = await (await fetch('/ne.json')).json();
    const base = { land: ne.land.map(decode), lakes: ne.lakes.map(decode), borders: ne.borders.map(decode), states: (ne.states ?? []).map(decode), places: [] };
    const rings = await (await fetch('/cells.json')).json();
    // Each cell is its own layer, as the map builds them, and compacted cells
    // are dashed, which is the costlier stroke of the two.
    const layers = rings.map((ring) => ({ kind: 'polygon', role: 'input', rings: [ring], cell: 'x', compacted: true }));
    const colors = { bg: 'rgb(243, 244, 241)', surface: 'rgb(255, 255, 255)', land: 'rgb(232, 235, 228)', line: 'rgb(201, 205, 196)', graticule: 'rgb(223, 226, 218)', muted: 'rgb(91, 97, 110)', text: 'rgb(21, 23, 28)', accent: 'rgb(181, 60, 10)', shadow: 'rgb(0, 0, 0)', sans: 'sans-serif' };
    const canvas = document.querySelector('canvas');
    const g = canvas.getContext('2d');
    const out = {};
    for (const mode of ['map', 'globe']) {
      const v0 = frame(mode, rings.flat().filter((_, i) => i % 97 === 0), 1280, 800);
      const times = [];
      for (let f = 0; f < 60; f++) {
        const view = { ...v0, lon: v0.lon + f * 0.002, width: 1280, height: 800 };
        const t0 = performance.now();
        draw(g, view, base, layers, colors);
        g.getImageData(0, 0, 1, 1); // make the frame's drawing actually finish
        times.push(performance.now() - t0);
      }
      times.sort((a, b) => a - b);
      out[mode] = { p50: times[29], p95: times[Math.ceil(0.95 * 60) - 1] };
    }
    return out;
  });
  t.diagnostic(`${(await cpuRate(browser, profile)).rate}x CPU (host BenchmarkIndex ${(await cpuRate(browser, profile)).hostIndex}, target ${profile.cpu.targetBenchmarkIndex}): map p50 ${result.map.p50.toFixed(1)} ms, p95 ${result.map.p95.toFixed(1)} ms · globe p50 ${result.globe.p50.toFixed(1)} ms, p95 ${result.globe.p95.toFixed(1)} ms`);
  for (const mode of ['map', 'globe']) {
    assert.ok(result[mode].p95 <= CEILING, `${mode}: p95 ${result[mode].p95.toFixed(1)} ms, over the ${CEILING} ms this is held to`);
  }
});
