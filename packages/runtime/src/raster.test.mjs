import { test } from 'node:test';
import assert from 'node:assert/strict';

import { processRasterScene } from './raster.mjs';

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
  const out = await processRasterScene(runtime, {
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

test('a resampled scene uses the chosen target dimensions and forwards the method', async () => {
  const inspected = [];
  const calls = [];
  const runtime = {
    inspectFile: async (_tool, handle) => {
      inspected.push(handle);
      return { ok: true, metadata: { width: 2, height: 1, bands: 1 } };
    },
    processFileWindow: async (...args) => {
      calls.push(args);
      return { ok: true, raster: { samples: [0.5, 0.6], valid: [true, true] } };
    },
  };
  const resampling = { method: 'bilinear', target: 'right' };
  const out = await processRasterScene(runtime, {
    ...options,
    windowWidth: 2,
    windowHeight: 1,
    resampling,
    accept() {},
  });
  assert.deepEqual(inspected, ['red']);
  assert.deepEqual(calls[0].at(-2), resampling);
  assert.equal(out.metadata.width, 2);
});

test('a scene forwards scalar kernel parameters with every window', async () => {
  const parametersSeen = [];
  const runtime = {
    inspectFile: async () => ({ ok: true, metadata: { width: 2, height: 1, bands: 1 } }),
    processFileWindow: async (...args) => {
      parametersSeen.push(args[8]);
      return { ok: true, raster: { samples: [0.4, 0.5], valid: [true, true] } };
    },
  };
  await processRasterScene(runtime, {
    ...options,
    windowWidth: 1,
    windowHeight: 1,
    parameters: { soil_factor: 0.25 },
    accept() {},
  });
  assert.deepEqual(parametersSeen, [{ soil_factor: 0.25 }, { soil_factor: 0.25 }]);
});

test('a scene forwards 3 ordered band sources to the multi-source host method', async () => {
  const calls = [];
  const runtime = {
    inspectFile: async () => ({ ok: true, metadata: { width: 1, height: 1, bands: 1 } }),
    processFileSources: async (...args) => {
      calls.push(args);
      return { ok: true, raster: { samples: [0.5], valid: [true] } };
    },
  };
  const sources = [
    { handle: 'nir', band: 0 },
    { handle: 'red', band: 1 },
    { handle: 'blue', band: 2 },
  ];
  const out = await processRasterScene(runtime, {
    tool: 'raster.index.evi',
    sources,
    windowWidth: 1,
    windowHeight: 1,
    maxSamples: 8,
    accept() {},
  });
  assert.equal(out.ok, true);
  assert.deepEqual(calls[0].slice(0, 2), ['raster.index.evi', sources]);
});

test('a dynamic 2-band scene uses the ordered-source host method', async () => {
  let called = false;
  const runtime = {
    inspectFile: async () => ({ ok: true, metadata: { width: 1, height: 1, bands: 1 } }),
    processFileSources: async () => {
      called = true;
      return { ok: true, raster: { samples: [0.5], valid: [true] } };
    },
  };
  const out = await processRasterScene(runtime, {
    tool: 'raster.index.band-math',
    sources: [{ handle: 'nir', band: 0 }, { handle: 'red', band: 0 }],
    dynamicBands: true,
    windowWidth: 1,
    windowHeight: 1,
    maxSamples: 2,
    accept() {},
  });
  assert.equal(out.ok, true);
  assert.equal(called, true);
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
  const out = await processRasterScene(runtime, {
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
  const out = await processRasterScene(runtime, {
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
  const failed = await processRasterScene(failedHost, {
    ...options,
    accept: (window) => accepted.push(window),
  });
  assert.equal(failed.error.code, 'INVALID_INPUT');
  assert.equal(accepted.length, 2);
  assert.equal(failedHost.calls.length, 3);

  const controller = new AbortController();
  const canceledHost = host();
  const canceled = await processRasterScene(canceledHost, {
    ...options,
    signal: controller.signal,
    accept: () => controller.abort(),
  });
  assert.equal(canceled, null);
  assert.equal(canceledHost.calls.length, 1);
});

test('scene bounds and the required consumer are validated before processing', async () => {
  await assert.rejects(
    () => processRasterScene(host(), { ...options, windowWidth: 0, accept() {} }),
    /windowWidth must be a positive integer/,
  );
  await assert.rejects(
    () => processRasterScene(host(), options),
    /accept must consume/,
  );
});

test('scene dimensions and window counts are bounded before pixel work starts', async () => {
  const tooLarge = host();
  tooLarge.inspectFile = async () => ({
    ok: true,
    metadata: { width: 500_000_001, height: 1, bands: 1 },
  });
  const size = await processRasterScene(tooLarge, { ...options, accept() {} });
  assert.equal(size.error.code, 'LIMIT_EXCEEDED');
  assert.match(size.error.message, /500000001 pixels/);
  assert.equal(tooLarge.calls.length, 0);

  const tooMany = host();
  tooMany.inspectFile = async () => ({
    ok: true,
    metadata: { width: 1_000_001, height: 1, bands: 1 },
  });
  const windows = await processRasterScene(tooMany, {
    ...options,
    windowWidth: 1,
    windowHeight: 1,
    accept() {},
  });
  assert.equal(windows.error.code, 'LIMIT_EXCEEDED');
  assert.match(windows.error.message, /1000001 windows/);
  assert.equal(tooMany.calls.length, 0);
});
