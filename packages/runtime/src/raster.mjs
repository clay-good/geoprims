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

export function parseRasterExportPlan(bytes) {
  if (!(bytes instanceof Uint8Array) || bytes.length < 32) throw new TypeError('The raster export plan is incomplete.');
  if (new TextDecoder().decode(bytes.subarray(0, 4)) !== 'GPR1') throw new TypeError('The raster export plan has an unknown version.');
  const view = new DataView(bytes.buffer, bytes.byteOffset, 32);
  const plan = {
    tileWidth: view.getUint32(4, true),
    tileHeight: view.getUint32(8, true),
    tilesAcross: view.getUint32(12, true),
    tilesDown: view.getUint32(16, true),
    tileBytes: view.getUint32(20, true),
    fileSize: Number(view.getBigUint64(24, true)),
    header: bytes.subarray(32),
  };
  for (const [name, value] of Object.entries(plan)) {
    if (name !== 'header') positiveInteger(value, name);
  }
  if (plan.tilesAcross * plan.tilesDown > MAX_SCENE_WINDOWS) throw new RangeError('The raster export plan has too many tiles.');
  if (plan.header.length + plan.tilesAcross * plan.tilesDown * plan.tileBytes !== plan.fileSize) {
    throw new TypeError('The raster export plan size does not match its header and tiles.');
  }
  return plan;
}

/**
 * Recomputes a scene as fixed row-major BigTIFF tiles. `accept` receives the
 * complete header first, then one bounded binary tile at a time.
 */
export async function exportRasterScene(host, options) {
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
  const sources = options.sources ?? [
    { handle: leftHandle, band: leftBand },
    { handle: rightHandle, band: rightBand },
  ];
  const useSourceList = sources.length !== 2 || options.dynamicBands;
  const tileWidth = positiveInteger(options.tileWidth, 'tileWidth');
  const tileHeight = positiveInteger(options.tileHeight, 'tileHeight');
  const maxSamples = positiveInteger(options.maxSamples, 'maxSamples');
  if (typeof accept !== 'function') throw new TypeError('accept must consume each GeoTIFF chunk.');
  if (sources.length < 2) throw new TypeError('sources must contain at least 2 raster bands.');
  if (sources.length > 2 && resampling) throw new TypeError('resampling is available only for 2-band scenes.');

  const targetHandle = resampling?.target === 'right' ? sources[1].handle : sources[0].handle;
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
  const planned = resultObject(await host.exportFilePlan(tool, targetHandle, tileWidth, tileHeight));
  if (!planned.ok) return planned;
  const plan = parseRasterExportPlan(planned.bytes);
  if (plan.tilesAcross !== Math.ceil(width / tileWidth) || plan.tilesDown !== Math.ceil(height / tileHeight)) {
    throw new TypeError('The raster export plan does not match the selected output grid.');
  }
  if (signal?.aborted) return null;
  await accept(plan.header);
  const total = plan.tilesAcross * plan.tilesDown;
  let completed = 0;
  for (let y = 0; y < height; y += tileHeight) {
    for (let x = 0; x < width; x += tileWidth) {
      if (signal?.aborted) return null;
      const window = {
        x,
        y,
        width: Math.min(tileWidth, width - x),
        height: Math.min(tileHeight, height - y),
      };
      const request = {
        tool,
        ...window,
        maxSamples,
        tileWidth,
        tileHeight,
        ...(parameters ? { parameters } : {}),
        ...(resampling ? { resampling } : {}),
        ...(useSourceList ? { sources } : {
          leftHandle: sources[0].handle,
          leftBand: sources[0].band,
          rightHandle: sources[1].handle,
          rightBand: sources[1].band,
        }),
      };
      const out = resultObject(await host.exportFileTile(tool, request));
      if (!out.ok) return out;
      if (!(out.bytes instanceof Uint8Array) || out.bytes.length !== plan.tileBytes) {
        throw new TypeError('The raster export tile does not match its plan.');
      }
      await accept(out.bytes);
      completed++;
      onProgress?.({ completed, total, window });
    }
  }
  return {
    ok: true,
    metadata: { ...inspected.metadata, bands: 1 },
    windows: completed,
    fileSize: plan.fileSize,
  };
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
  const sources = options.sources ?? [
    { handle: leftHandle, band: leftBand },
    { handle: rightHandle, band: rightBand },
  ];
  const useSourceList = sources.length !== 2 || options.dynamicBands;
  const windowWidth = positiveInteger(options.windowWidth, 'windowWidth');
  const windowHeight = positiveInteger(options.windowHeight, 'windowHeight');
  const maxSamples = positiveInteger(options.maxSamples, 'maxSamples');
  if (typeof accept !== 'function') throw new TypeError('accept must consume each processed raster window.');

  if (sources.length < 2) throw new TypeError('sources must contain at least 2 raster bands.');
  if (sources.length > 2 && resampling) throw new TypeError('resampling is available only for 2-band scenes.');
  const targetHandle = resampling?.target === 'right' ? sources[1].handle : sources[0].handle;
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
      const out = resultObject(!useSourceList
        ? await host.processFileWindow(
            tool,
            sources[0].handle, sources[0].band,
            sources[1].handle, sources[1].band,
            window,
            maxSamples,
            resampling,
            parameters,
          )
        : await host.processFileSources(tool, sources, window, maxSamples, parameters));
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
