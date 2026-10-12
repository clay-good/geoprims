const resultObject = (value) => (typeof value === 'string' ? JSON.parse(value) : value);

function positiveInteger(value, name) {
  if (!Number.isSafeInteger(value) || value <= 0) throw new TypeError(`${name} must be a positive integer.`);
  return value;
}

/**
 * Processes a complete pair of matching local rasters in bounded row-major
 * windows. `accept` consumes each successful Rust result before the next
 * window starts, so neither source size nor output size becomes a memory bound.
 */
export async function processNormalizedDifferenceScene(host, options) {
  const {
    tool,
    leftHandle,
    leftBand,
    rightHandle,
    rightBand,
    accept,
    signal,
    onProgress,
  } = options;
  const windowWidth = positiveInteger(options.windowWidth, 'windowWidth');
  const windowHeight = positiveInteger(options.windowHeight, 'windowHeight');
  const maxSamples = positiveInteger(options.maxSamples, 'maxSamples');
  if (typeof accept !== 'function') throw new TypeError('accept must consume each processed raster window.');

  const inspected = resultObject(await host.inspectFile(tool, leftHandle));
  if (!inspected.ok) return inspected;
  const { width, height } = inspected.metadata;
  const across = Math.ceil(width / windowWidth);
  const down = Math.ceil(height / windowHeight);
  const total = across * down;
  let completed = 0;

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
      ));
      if (!out.ok) return out;
      await accept({ window, raster: out.raster });
      completed++;
      onProgress?.({ completed, total, window });
    }
  }
  return {
    ok: true,
    metadata: { ...inspected.metadata, bands: 1 },
    windows: completed,
  };
}
