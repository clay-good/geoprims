// Pixel-level regression fixture for the map's core layer vocabulary. Geometry
// tests prove where layers belong; this proves the renderer keeps their visual
// roles distinct: context is unfilled, results carry the accent, and selected
// cells use a light accent wash rather than reading as another solid area.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { web } from './site.mjs';

const FILES = { '/projection.js': 'src/lib/map/projection.js', '/render.js': 'src/lib/map/render.js' };

test('map layer roles keep their pixel-level visual treatment', { timeout: 120_000 }, async (t) => {
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const page = await browser.newPage({ viewport: { width: 240, height: 160 }, deviceScaleFactor: 1 });
  await page.route('http://map-visual.local/**', (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path === '/') return route.fulfill({ contentType: 'text/html', body: '<!doctype html><canvas width="240" height="160"></canvas>' });
    const file = FILES[path];
    return file ? route.fulfill({ contentType: 'text/javascript', body: readFileSync(join(web, file)) }) : route.fulfill({ status: 404 });
  });
  await page.goto('http://map-visual.local/');

  const pixels = await page.evaluate(async () => {
    const { draw } = await import('/render.js');
    const canvas = document.querySelector('canvas');
    const g = canvas.getContext('2d');
    const colors = {
      bg: 'rgb(243, 244, 241)', surface: 'rgb(255, 255, 255)', land: 'rgb(232, 235, 228)',
      line: 'rgb(201, 205, 196)', graticule: 'rgb(223, 226, 218)', muted: 'rgb(91, 97, 110)',
      text: 'rgb(21, 23, 28)', accent: 'rgb(181, 60, 10)', shadow: 'rgb(0, 0, 0)', sans: 'sans-serif',
    };
    const layers = [
      { kind: 'polygon', role: 'input', grid: true, gridKind: 'h3', rings: [[[-90, -50], [-30, -50], [-30, 10], [-90, 10]]] },
      { kind: 'polygon', role: 'result', cell: 'selected', rings: [[[10, -50], [70, -50], [70, 10], [10, 10]]] },
      { kind: 'line', role: 'result', points: [[-90, 50], [70, 50]] },
      { kind: 'point', role: 'result', points: [[0, 0]] },
    ];
    // One degree is one pixel, and moving suppresses contextual graticule and
    // casings so every sampled pixel belongs to the layer named beside it.
    draw(g, { mode: 'equirect', lon: 0, lat: 0, scale: 180 / Math.PI, width: 240, height: 160, moving: true }, null, layers, colors);
    const at = (x, y) => [...g.getImageData(x, y, 1, 1).data];
    return {
      background: at(15, 100),
      gridInterior: at(60, 100),
      gridEdge: at(30, 100),
      selectedCell: at(160, 100),
      resultLine: at(100, 30),
      resultPoint: at(125, 80),
    };
  });

  const near = (actual, expected, label) => {
    assert.ok(actual.every((value, i) => Math.abs(value - expected[i]) <= 2), `${label}: ${actual} != ${expected}`);
  };
  const background = [243, 244, 241, 255];
  assert.deepEqual(pixels.background, background);
  assert.deepEqual(pixels.gridInterior, background, 'grid context has no fill');
  assert.notDeepEqual(pixels.gridEdge, background, 'grid context still has an outline');
  near(pixels.selectedCell, [226, 192, 176, 255], 'selected cell accent wash');
  near(pixels.resultLine, [181, 60, 10, 255], 'result line accent');
  near(pixels.resultPoint, [181, 60, 10, 255], 'result point accent ring');
});
