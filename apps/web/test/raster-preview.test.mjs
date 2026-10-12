import assert from 'node:assert/strict';
import test from 'node:test';

import { previewHistogram, previewPixels, previewReadout, rasterPreview } from '../src/lib/raster-preview.mjs';

test('a scene preview samples bounded pixels across window edges', () => {
  const preview = rasterPreview(4, 2, 2);
  preview.add({
    window: { x: 0, y: 0, width: 2, height: 2 },
    raster: { samples: [0, 1, 4, 5], valid: [true, true, true, true] },
  });
  preview.add({
    window: { x: 2, y: 0, width: 2, height: 2 },
    raster: { samples: [2, 3, 6, 7], valid: [true, false, true, true] },
  });
  const out = preview.value();
  assert.equal(out.width, 2);
  assert.equal(out.height, 1);
  assert.deepEqual([...out.columns], [1, 3]);
  assert.deepEqual([...out.rows], [1]);
  assert.deepEqual([...out.samples], [5, 7]);
  assert.deepEqual([...out.valid], [1, 1]);
});

test('preview colors center a signed range and mark no-data', () => {
  const preview = {
    samples: new Float64Array([-1, 0, 1, 999]),
    valid: new Uint8Array([1, 1, 1, 0]),
  };
  assert.deepEqual([...previewPixels(preview, -1, 1)], [
    35, 90, 160, 255,
    240, 240, 235, 255,
    30, 125, 95, 255,
    218, 218, 218, 255,
  ]);
});

test('the preview histogram is bounded, bins edge values, and excludes no-data', () => {
  const histogram = previewHistogram({
    samples: new Float64Array([-1, -0.5, 0, 0.5, 1, 999]),
    valid: new Uint8Array([1, 1, 1, 1, 1, 0]),
  }, -1, 1, 4);
  assert.deepEqual([...histogram.counts], [1, 1, 1, 2]);
  assert.deepEqual([...histogram.edges], [-1, -0.5, 0, 0.5, 1]);
  assert.equal(histogram.sampledPixels, 5);
  assert.equal(histogram.maxCount, 2);
  assert.throws(() => previewHistogram({ samples: [], valid: [] }, 0, 1, 1), /2 through 256/);
});

test('cursor readout uses the source pixel center and affine transform', () => {
  const preview = rasterPreview(4, 2, 2);
  preview.add({
    window: { x: 0, y: 0, width: 4, height: 2 },
    raster: { samples: [0, 1, 2, 3, 4, 5, 6, 7], valid: Array(8).fill(true) },
  });
  assert.deepEqual(previewReadout(preview.value(), {
    epsg: 32617,
    rasterType: 'area',
    transform: { x0: 100, dx: 10, rx: 0, y0: 200, ry: 0, dy: -10 },
  }, 1, 0), {
    column: 3,
    row: 1,
    value: 7,
    x: 135,
    y: 185,
    epsg: 32617,
  });
});
