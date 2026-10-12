import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { clientTool, tool } from '../src/lib/catalog.mjs';

const component = await readFile(new URL('../src/components/RasterPanel.svelte', import.meta.url), 'utf8');
const toolApp = await readFile(new URL('../src/components/ToolApp.svelte', import.meta.url), 'utf8');

test('only tools with a raster manifest contract load the local-file panel', () => {
  assert.match(toolApp, /tool\['x-raster'\]/);
  assert.match(toolApp, /import\('\.\/RasterPanel\.svelte'\)/);
  assert.deepEqual(clientTool(tool('raster.index.ndvi'))['x-raster'], { bands: ['nir', 'red'], output: 'ndvi' });
});

test('the panel accepts one multiband TIFF or separate TIFFs and maps every role', () => {
  assert.match(component, /accept="\.tif,\.tiff,image\/tiff" multiple/);
  assert.match(component, /tool\['x-raster'\]\.bands/);
  assert.match(component, /Band number/);
  assert.match(component, /processFileScene/);
});

test('private raster state is not persisted or sent', () => {
  assert.match(component, /not put in links or reports/);
  assert.doesNotMatch(component, /encodeLink|localStorage|sessionStorage|fetch\(/);
});

test('the completed scene shows a bounded preview, exact statistics, and cursor readout', () => {
  assert.match(component, /rasterPreview/);
  assert.match(component, /at most 512 × 512 pixels/);
  assert.match(component, /Statistics use every valid output pixel/);
  assert.match(component, /previewReadout/);
});

test('the preview includes numeric legend ticks and an accessible sampled histogram', () => {
  assert.match(component, /previewHistogram/);
  assert.match(component, /role="img" aria-label={`Histogram/);
  assert.match(component, /<dl class="sr-only">/);
  assert.match(component, /Preview histogram values/);
});
