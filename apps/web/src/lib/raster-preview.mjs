export const MAX_PREVIEW_SIDE = 512;

/** A bounded nearest-sample preview assembled from row-major output windows. */
export function rasterPreview(width, height, maxSide = MAX_PREVIEW_SIDE) {
  if (![width, height, maxSide].every((v) => Number.isSafeInteger(v) && v > 0)) {
    throw new TypeError('Raster preview dimensions must be positive integers.');
  }
  const scale = Math.min(1, maxSide / width, maxSide / height);
  const previewWidth = Math.max(1, Math.round(width * scale));
  const previewHeight = Math.max(1, Math.round(height * scale));
  const columns = Int32Array.from(
    { length: previewWidth },
    (_, x) => Math.min(width - 1, Math.floor(((x + 0.5) * width) / previewWidth)),
  );
  const rows = Int32Array.from(
    { length: previewHeight },
    (_, y) => Math.min(height - 1, Math.floor(((y + 0.5) * height) / previewHeight)),
  );
  const samples = new Float64Array(previewWidth * previewHeight);
  const valid = new Uint8Array(samples.length);

  return {
    add({ window, raster }) {
      for (let py = 0; py < previewHeight; py++) {
        const sourceY = rows[py];
        if (sourceY < window.y || sourceY >= window.y + window.height) continue;
        for (let px = 0; px < previewWidth; px++) {
          const sourceX = columns[px];
          if (sourceX < window.x || sourceX >= window.x + window.width) continue;
          const source = (sourceY - window.y) * window.width + sourceX - window.x;
          const target = py * previewWidth + px;
          samples[target] = raster.samples[source];
          valid[target] = raster.valid[source] ? 1 : 0;
        }
      }
    },
    value() {
      return { width: previewWidth, height: previewHeight, sceneWidth: width, sceneHeight: height, columns, rows, samples, valid };
    },
  };
}

const mix = (a, b, t) => Math.round(a + (b - a) * t);

/** Color-vision-safe blue-to-neutral-to-green pixels for the preview canvas. */
export function previewPixels(preview, min, max) {
  const rgba = new Uint8ClampedArray(preview.samples.length * 4);
  const crossesZero = min < 0 && max > 0;
  const extent = crossesZero ? Math.max(-min, max) : max - min;
  for (let i = 0; i < preview.samples.length; i++) {
    const p = i * 4;
    if (!preview.valid[i]) {
      rgba.set([218, 218, 218, 255], p);
      continue;
    }
    const raw = extent === 0 ? 0.5 : crossesZero
      ? (preview.samples[i] / extent + 1) / 2
      : (preview.samples[i] - min) / extent;
    const t = Math.max(0, Math.min(1, raw));
    const left = t <= 0.5;
    const q = left ? t * 2 : (t - 0.5) * 2;
    const from = left ? [35, 90, 160] : [240, 240, 235];
    const to = left ? [240, 240, 235] : [30, 125, 95];
    rgba[p] = mix(from[0], to[0], q);
    rgba[p + 1] = mix(from[1], to[1], q);
    rgba[p + 2] = mix(from[2], to[2], q);
    rgba[p + 3] = 255;
  }
  return rgba;
}

/** The source pixel and model coordinate under one preview pixel. */
export function previewReadout(preview, metadata, x, y) {
  if (!Number.isInteger(x) || !Number.isInteger(y) || x < 0 || y < 0 || x >= preview.width || y >= preview.height) return null;
  const i = y * preview.width + x;
  const column = preview.columns[x];
  const row = preview.rows[y];
  const out = { column, row, value: preview.valid[i] ? preview.samples[i] : null };
  const t = metadata.transform;
  if (!t) return out;
  const offset = metadata.rasterType === 'point' ? 0 : 0.5;
  const px = column + offset;
  const py = row + offset;
  return {
    ...out,
    x: t.x0 + px * t.dx + py * t.rx,
    y: t.y0 + px * t.ry + py * t.dy,
    epsg: metadata.epsg ?? null,
  };
}
