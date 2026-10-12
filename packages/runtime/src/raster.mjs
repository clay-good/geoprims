const resultObject = (value) => (typeof value === 'string' ? JSON.parse(value) : value);
export const MAX_SCENE_PIXELS = 500_000_000;
export const MAX_SCENE_WINDOWS = 1_000_000;

function sceneSummary() {
  let validPixels = 0;
  let noDataPixels = 0;
  let min = Infinity;
  let max = -Infinity;
  let sum = 0;
  let compensation = 0;
  return {
    add(raster) {
      for (let i = 0; i < raster.samples.length; i++) {
        if (!raster.valid[i]) {
          noDataPixels++;
          continue;
        }
        const value = raster.samples[i];
        validPixels++;
        min = Math.min(min, value);
        max = Math.max(max, value);
        // Neumaier summation keeps the mean stable when values have very
        // different magnitudes, without retaining any scene pixels.
        const next = sum + value;
        compensation += Math.abs(sum) >= Math.abs(value)
          ? (sum - next) + value
          : (value - next) + sum;
        sum = next;
      }
    },
    value() {
      return {
        validPixels,
        noDataPixels,
        min: validPixels ? min : null,
        max: validPixels ? max : null,
        mean: validPixels ? (sum + compensation) / validPixels : null,
      };
    },
  };
}

function positiveInteger(value, name) {
  if (!Number.isSafeInteger(value) || value <= 0) throw new TypeError(`${name} must be a positive integer.`);
  return value;
}

/**
 * Processes a complete pair of matching local rasters in bounded row-major
 * windows. `accept` consumes each successful Rust result before the next
 * window starts, so neither source size nor output size becomes a memory bound.
 */
export async function processRasterScene(host, options) {
  const {
    tool,
    leftHandle,
    leftBand,
    rightHandle,
    rightBand,
    accept,
    signal,
    onProgress,
    resampling,
    parameters,
  } = options;
  const windowWidth = positiveInteger(options.windowWidth, 'windowWidth');
  const windowHeight = positiveInteger(options.windowHeight, 'windowHeight');
  const maxSamples = positiveInteger(options.maxSamples, 'maxSamples');
  if (typeof accept !== 'function') throw new TypeError('accept must consume each processed raster window.');

  const targetHandle = resampling?.target === 'right' ? rightHandle : leftHandle;
  const inspected = resultObject(await host.inspectFile(tool, targetHandle));
  if (!inspected.ok) return inspected;
  const { width, height } = inspected.metadata;
  positiveInteger(width, 'raster width');
  positiveInteger(height, 'raster height');
  const pixels = width * height;
  if (!Number.isSafeInteger(pixels) || pixels > MAX_SCENE_PIXELS) {
    return {
      ok: false,
      error: {
        code: 'LIMIT_EXCEEDED',
        message: `The raster has ${Number.isSafeInteger(pixels) ? pixels : 'too many'} pixels; this surface allows ${MAX_SCENE_PIXELS}.`,
      },
    };
  }
  const across = Math.ceil(width / windowWidth);
  const down = Math.ceil(height / windowHeight);
  const total = across * down;
  if (!Number.isSafeInteger(total) || total > MAX_SCENE_WINDOWS) {
    return {
      ok: false,
      error: {
        code: 'LIMIT_EXCEEDED',
        message: `The selected window size would require ${Number.isSafeInteger(total) ? total : 'too many'} windows; this surface allows ${MAX_SCENE_WINDOWS}.`,
      },
    };
  }
  let completed = 0;
  const summary = sceneSummary();

  for (let y = 0; y < height; y += windowHeight) {
    for (let x = 0; x < width; x += windowWidth) {
      if (signal?.aborted) return null;
      const window = {
        x,
        y,
        width: Math.min(windowWidth, width - x),
        height: Math.min(windowHeight, height - y),
      };
      const out = resultObject(await host.processFileWindow(
        tool,
        leftHandle, leftBand,
        rightHandle, rightBand,
        window,
        maxSamples,
        resampling,
        parameters,
      ));
      if (!out.ok) return out;
      await accept({ window, raster: out.raster });
      summary.add(out.raster);
      completed++;
      onProgress?.({ completed, total, window });
    }
  }
  return {
    ok: true,
    metadata: { ...inspected.metadata, bands: 1 },
    windows: completed,
    summary: summary.value(),
  };
}
