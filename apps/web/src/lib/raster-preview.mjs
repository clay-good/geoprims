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

const linear = (channel) => {
  const value = channel / 255;
  return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
};

const oklab = ([red, green, blue]) => {
  const r = linear(red);
  const g = linear(green);
  const b = linear(blue);
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  return [
    0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
    1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
    0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
  ];
};

const srgb = (value) => {
  const channel = value <= 0.0031308 ? 12.92 * value : 1.055 * value ** (1 / 2.4) - 0.055;
  return Math.round(Math.max(0, Math.min(1, channel)) * 255);
};

const perceptualMix = (from, to, t) => {
  const a = oklab(from);
  const b = oklab(to);
  const L = a[0] + (b[0] - a[0]) * t;
  const A = a[1] + (b[1] - a[1]) * t;
  const B = a[2] + (b[2] - a[2]) * t;
  const l = (L + 0.3963377774 * A + 0.2158037573 * B) ** 3;
  const m = (L - 0.1055613458 * A - 0.0638541728 * B) ** 3;
  const s = (L - 0.0894841775 * A - 1.291485548 * B) ** 3;
  return [
    srgb(4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s),
    srgb(-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s),
    srgb(-0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s),
  ];
};

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
    const color = perceptualMix(from, to, q);
    rgba[p] = color[0];
    rgba[p + 1] = color[1];
    rgba[p + 2] = color[2];
    rgba[p + 3] = 255;
  }
  return rgba;
}

/** A bounded histogram of the pixels retained for the visual preview. */
export function previewHistogram(preview, min, max, binCount = 32) {
  if (!Number.isSafeInteger(binCount) || binCount < 2 || binCount > 256) {
    throw new TypeError('Raster histogram bins must be an integer from 2 through 256.');
  }
  const counts = new Uint32Array(binCount);
  let sampledPixels = 0;
  const extent = max - min;
  for (let i = 0; i < preview.samples.length; i++) {
    if (!preview.valid[i]) continue;
    const raw = extent === 0 ? 0 : (preview.samples[i] - min) / extent;
    const bin = Math.min(binCount - 1, Math.max(0, Math.floor(raw * binCount)));
    counts[bin]++;
    sampledPixels++;
  }
  return {
    counts,
    sampledPixels,
    maxCount: counts.reduce((largest, count) => Math.max(largest, count), 0),
    edges: Float64Array.from({ length: binCount + 1 }, (_, i) => min + (extent * i) / binCount),
  };
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
