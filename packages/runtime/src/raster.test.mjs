import { test } from 'node:test';
import assert from 'node:assert/strict';

import { processNormalizedDifferenceScene } from './raster.mjs';

function host({ failAt } = {}) {
  const calls = [];
  return {
    calls,
    inspectFile: async () => JSON.stringify({
      ok: true,
      metadata: { width: 5, height: 3, bands: 2, epsg: 32617 },
    }),
    processFileWindow: async (...args) => {
      const window = args[5];
      calls.push(window);
      if (calls.length === failAt) return JSON.stringify({ ok: false, error: { code: 'INVALID_INPUT' } });
      return JSON.stringify({ ok: true, raster: { samples: [calls.length], valid: [true] } });
    },
  };
}

const options = {
  tool: 'raster.index.ndvi',
  leftHandle: 'nir',
  leftBand: 0,
  rightHandle: 'red',
  rightBand: 1,
  windowWidth: 2,
  windowHeight: 2,
  maxSamples: 8,
};

test('a raster scene is consumed one successful window at a time in row-major order', async () => {
  const runtime = host();
  const accepted = [];
  const progress = [];
  const out = await processNormalizedDifferenceScene(runtime, {
    ...options,
    accept: (window) => accepted.push(window),
    onProgress: (state) => progress.push(state),
  });
  assert.deepEqual(runtime.calls, [
    { x: 0, y: 0, width: 2, height: 2 },
    { x: 2, y: 0, width: 2, height: 2 },
    { x: 4, y: 0, width: 1, height: 2 },
    { x: 0, y: 2, width: 2, height: 1 },
    { x: 2, y: 2, width: 2, height: 1 },
    { x: 4, y: 2, width: 1, height: 1 },
  ]);
  assert.equal(accepted.length, 6);
  assert.equal(progress.at(-1).completed, 6);
  assert.equal(progress.at(-1).total, 6);
  assert.deepEqual(out, {
    ok: true,
    metadata: { width: 5, height: 3, bands: 1, epsg: 32617 },
    windows: 6,
    summary: { validPixels: 6, noDataPixels: 0, min: 1, max: 6, mean: 3.5 },
  });
});

test('scene statistics stream across windows and exclude no-data pixels', async () => {
  let window = 0;
  const runtime = {
    inspectFile: async () => ({ ok: true, metadata: { width: 4, height: 1, bands: 1 } }),
    processFileWindow: async () => {
      window++;
      return window === 1
        ? { ok: true, raster: { samples: [1e16, 1], valid: [true, true] } }
        : { ok: true, raster: { samples: [-1e16, -9999], valid: [true, false] } };
    },
  };
  const out = await processNormalizedDifferenceScene(runtime, {
    ...options,
    windowWidth: 2,
    windowHeight: 1,
    accept() {},
  });
  assert.deepEqual(out.summary, {
    validPixels: 3,
    noDataPixels: 1,
    min: -1e16,
    max: 1e16,
    mean: 1 / 3,
  });
});

test('an all-no-data scene has counts but no numeric statistics', async () => {
  const runtime = {
    inspectFile: async () => ({ ok: true, metadata: { width: 1, height: 1, bands: 1 } }),
    processFileWindow: async () => ({
      ok: true,
      raster: { samples: [-9999], valid: [false] },
    }),
  };
  const out = await processNormalizedDifferenceScene(runtime, {
    ...options,
    windowWidth: 1,
    windowHeight: 1,
    accept() {},
  });
  assert.deepEqual(out.summary, {
    validPixels: 0,
    noDataPixels: 1,
    min: null,
    max: null,
    mean: null,
  });
});

test('a failed or canceled scene emits no later windows', async () => {
  const failedHost = host({ failAt: 3 });
  const accepted = [];
  const failed = await processNormalizedDifferenceScene(failedHost, {
    ...options,
    accept: (window) => accepted.push(window),
  });
  assert.equal(failed.error.code, 'INVALID_INPUT');
  assert.equal(accepted.length, 2);
  assert.equal(failedHost.calls.length, 3);

  const controller = new AbortController();
  const canceledHost = host();
  const canceled = await processNormalizedDifferenceScene(canceledHost, {
    ...options,
    signal: controller.signal,
    accept: () => controller.abort(),
  });
  assert.equal(canceled, null);
  assert.equal(canceledHost.calls.length, 1);
});

test('scene bounds and the required consumer are validated before processing', async () => {
  await assert.rejects(
    () => processNormalizedDifferenceScene(host(), { ...options, windowWidth: 0, accept() {} }),
    /windowWidth must be a positive integer/,
  );
  await assert.rejects(
    () => processNormalizedDifferenceScene(host(), options),
    /accept must consume/,
  );
});

test('scene dimensions and window counts are bounded before pixel work starts', async () => {
  const tooLarge = host();
  tooLarge.inspectFile = async () => ({
    ok: true,
    metadata: { width: 500_000_001, height: 1, bands: 1 },
  });
  const size = await processNormalizedDifferenceScene(tooLarge, { ...options, accept() {} });
  assert.equal(size.error.code, 'LIMIT_EXCEEDED');
  assert.match(size.error.message, /500000001 pixels/);
  assert.equal(tooLarge.calls.length, 0);

  const tooMany = host();
  tooMany.inspectFile = async () => ({
    ok: true,
    metadata: { width: 1_000_001, height: 1, bands: 1 },
  });
  const windows = await processNormalizedDifferenceScene(tooMany, {
    ...options,
    windowWidth: 1,
    windowHeight: 1,
    accept() {},
  });
  assert.equal(windows.error.code, 'LIMIT_EXCEEDED');
  assert.match(windows.error.message, /1000001 windows/);
  assert.equal(tooMany.calls.length, 0);
});
