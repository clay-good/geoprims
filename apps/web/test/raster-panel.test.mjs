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
  assert.deepEqual(clientTool(tool('raster.index.evi2'))['x-raster'], { bands: ['nir', 'red'], output: 'evi2' });
  assert.deepEqual(clientTool(tool('raster.index.savi'))['x-raster'], { bands: ['nir', 'red'], output: 'savi' });
  assert.deepEqual(clientTool(tool('raster.index.dnbr'))['x-raster'], { bands: ['nbr_pre', 'nbr_post'], output: 'dnbr' });
});

test('the panel accepts one multiband TIFF or separate TIFFs and maps every role', () => {
  assert.match(component, /accept="\.tif,\.tiff,image\/tiff" multiple/);
  assert.match(component, /tool\['x-raster'\]\.bands/);
  assert.match(component, /Band number/);
  assert.match(component, /processFileScene/);
  assert.match(component, /parameters: getInputs\(\)/);
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

test('a repairable grid mismatch offers both Rust resampling methods and target grids', () => {
  assert.match(component, /outcome\.error\.details\?\.reason !== 'CRS'/);
  assert.match(component, /<option value="nearest">Nearest neighbor<\/option>/);
  assert.match(component, /<option value="bilinear">Bilinear<\/option>/);
  assert.match(component, /<option value="left">/);
  assert.match(component, /<option value="right">/);
  assert.match(component, /Resample and process/);
  assert.match(component, /resampling: resample/);
});
