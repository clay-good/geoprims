// The home page's terrain: contour lines over a drifting height field, with a
// survey reticle that reads out the ground under it. Decoration only — it is
// aria-hidden, computes nothing a reader relies on, and draws a single still
// frame when the reader has asked for reduced motion.
//
// The field is seeded value noise, so every visit draws the same country.
// Contours come from marching squares over a coarse grid, which keeps a frame
// to a few milliseconds on a phone.

/** A small deterministic generator, so the terrain is the same every visit. */
function mulberry32(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** Smooth value noise on a 256-cell lattice, summed over a few octaves. */
export function heightField(seed = 1609) {
  const rand = mulberry32(seed);
  const N = 256;
  const lattice = Float32Array.from({ length: N * N }, rand);
  const fade = (t) => t * t * (3 - 2 * t);
  const at = (x, y) => lattice[((y & (N - 1)) * N) + (x & (N - 1))];
  const noise = (x, y) => {
    const xi = Math.floor(x);
    const yi = Math.floor(y);
    const u = fade(x - xi);
    const v = fade(y - yi);
    const a = at(xi, yi) + (at(xi + 1, yi) - at(xi, yi)) * u;
    const b = at(xi, yi + 1) + (at(xi + 1, yi + 1) - at(xi, yi + 1)) * u;
    return a + (b - a) * v;
  };
  return (x, y) => noise(x, y) * 0.6 + noise(x * 2.1 + 17, y * 2.1 + 3) * 0.28 + noise(x * 4.3 + 5, y * 4.3 + 41) * 0.12;
}

/**
 * The contour segments of a grid of heights at one level, by marching
 * squares: for each cell, the edge crossings that level makes.
 */
export function contourSegments(grid, cols, rows, level, cell) {
  const out = [];
  const lerp = (a, b) => (level - a) / (b - a);
  for (let j = 0; j < rows - 1; j += 1) {
    for (let i = 0; i < cols - 1; i += 1) {
      const a = grid[j * cols + i];
      const b = grid[j * cols + i + 1];
      const c = grid[(j + 1) * cols + i + 1];
      const d = grid[(j + 1) * cols + i];
      const k = (a > level ? 8 : 0) | (b > level ? 4 : 0) | (c > level ? 2 : 0) | (d > level ? 1 : 0);
      if (k === 0 || k === 15) continue;
      const x = i * cell;
      const y = j * cell;
      const top = [x + lerp(a, b) * cell, y];
      const right = [x + cell, y + lerp(b, c) * cell];
      const bottom = [x + lerp(d, c) * cell, y + cell];
      const left = [x, y + lerp(a, d) * cell];
      const pairs = {
        1: [[left, bottom]], 2: [[bottom, right]], 3: [[left, right]], 4: [[top, right]],
        5: [[left, top], [bottom, right]], 6: [[top, bottom]], 7: [[left, top]], 8: [[left, top]],
        9: [[top, bottom]], 10: [[left, bottom], [top, right]], 11: [[top, right]], 12: [[left, right]],
        13: [[bottom, right]], 14: [[left, bottom]],
      }[k];
      for (const p of pairs) out.push(p);
    }
  }
  return out;
}

/** Formats a coordinate the way a GNSS readout does. */
export const readoutCoord = (lat, lon) =>
  `${lat >= 0 ? 'N' : 'S'} ${Math.abs(lat).toFixed(5).padStart(8, '0')}°  ${lon >= 0 ? 'E' : 'W'} ${Math.abs(lon).toFixed(5).padStart(9, '0')}°`;

/**
 * Starts the terrain on a canvas. `readout` receives the reticle's position
 * and the height under it. Returns a stop function.
 */
export function startTerrain(canvas, readout) {
  const ctx = canvas.getContext('2d');
  if (!ctx) return () => {};
  const field = heightField();
  const still = matchMedia('(prefers-reduced-motion: reduce)').matches;
  const css = getComputedStyle(document.documentElement);
  const color = (name) => css.getPropertyValue(name).trim();
  let colors = { contour: color('--contour'), index: color('--contour-index'), signal: color('--signal'), data: color('--data') };
  const CELL = 10;
  const LEVELS = 14;
  let w = 0;
  let h = 0;
  let cols = 0;
  let rows = 0;
  let grid = new Float32Array(0);
  let pointer = null;
  let frame = 0;
  let running = true;
  let visible = true;
  let raf = 0;

  const resize = () => {
    const dpr = Math.min(devicePixelRatio || 1, 2);
    const box = canvas.getBoundingClientRect();
    w = Math.max(1, box.width);
    h = Math.max(1, box.height);
    canvas.width = Math.round(w * dpr);
    canvas.height = Math.round(h * dpr);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    cols = Math.ceil(w / CELL) + 1;
    rows = Math.ceil(h / CELL) + 1;
    grid = new Float32Array(cols * rows);
  };

  const draw = (t) => {
    // The country drifts slowly east; heights come from the field in grid units.
    const drift = still ? 0 : t * 0.000018;
    const scale = 0.0055;
    for (let j = 0; j < rows; j += 1) {
      for (let i = 0; i < cols; i += 1) {
        grid[j * cols + i] = field(i * CELL * scale + drift, j * CELL * scale + drift * 0.35);
      }
    }
    ctx.clearRect(0, 0, w, h);
    ctx.lineCap = 'round';
    for (let n = 1; n < LEVELS; n += 1) {
      const level = 0.15 + (n / LEVELS) * 0.7;
      const index = n % 5 === 0;
      ctx.strokeStyle = index ? colors.index : colors.contour;
      ctx.lineWidth = index ? 1.35 : 0.8;
      ctx.beginPath();
      for (const [[x1, y1], [x2, y2]] of contourSegments(grid, cols, rows, level, CELL)) {
        ctx.moveTo(x1, y1);
        ctx.lineTo(x2, y2);
      }
      ctx.stroke();
    }
    // The reticle follows the pointer, or walks a slow survey loop on its own.
    const auto = { x: w * (0.68 + 0.18 * Math.sin(t * 0.00023)), y: h * (0.52 + 0.22 * Math.sin(t * 0.00031 + 1)) };
    const at = pointer ?? auto;
    const r = 22;
    ctx.strokeStyle = colors.signal;
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.arc(at.x, at.y, r, 0, Math.PI * 2);
    for (const [dx, dy] of [[1, 0], [-1, 0], [0, 1], [0, -1]]) {
      ctx.moveTo(at.x + dx * (r - 6), at.y + dy * (r - 6));
      ctx.lineTo(at.x + dx * (r + 10), at.y + dy * (r + 10));
    }
    ctx.stroke();
    ctx.fillStyle = colors.signal;
    ctx.beginPath();
    ctx.arc(at.x, at.y, 2.2, 0, Math.PI * 2);
    ctx.fill();
    if (readout) {
      const i = Math.min(cols - 1, Math.max(0, Math.round(at.x / CELL)));
      const j = Math.min(rows - 1, Math.max(0, Math.round(at.y / CELL)));
      const height = grid[j * cols + i];
      // A made-up but plausible patch of the Front Range, so the readout reads like a GNSS.
      readout({ lat: 40.02 - (at.y / h) * 0.08, lon: -105.3 + (at.x / w) * 0.12, elevation: 1600 + height * 900 });
    }
  };

  const loop = (t) => {
    if (!running) return;
    frame += 1;
    // Every other frame is plenty for a drift this slow, and halves the cost.
    if (visible && frame % 2 === 0) draw(t);
    raf = requestAnimationFrame(loop);
  };

  const onPointer = (e) => {
    const box = canvas.getBoundingClientRect();
    pointer = { x: e.clientX - box.left, y: e.clientY - box.top };
    if (still) draw(0);
  };
  const onLeave = () => {
    pointer = null;
  };
  // The pointer moves over the hero's words, not the canvas behind them.
  const host = canvas.closest('.hero') ?? canvas.parentElement;
  host?.addEventListener('pointermove', onPointer, { passive: true });
  host?.addEventListener('pointerleave', onLeave);
  const observer = new IntersectionObserver(([entry]) => (visible = entry.isIntersecting));
  observer.observe(canvas);
  const onResize = () => {
    resize();
    draw(performance.now());
  };
  addEventListener('resize', onResize, { passive: true });
  // A theme change repaints in the new mode's colors.
  const themeWatch = new MutationObserver(() => {
    const next = getComputedStyle(document.documentElement);
    colors = { contour: next.getPropertyValue('--contour').trim(), index: next.getPropertyValue('--contour-index').trim(), signal: next.getPropertyValue('--signal').trim(), data: next.getPropertyValue('--data').trim() };
    draw(performance.now());
  });
  themeWatch.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });

  resize();
  draw(0);
  if (!still) raf = requestAnimationFrame(loop);

  return () => {
    running = false;
    cancelAnimationFrame(raf);
    observer.disconnect();
    themeWatch.disconnect();
    removeEventListener('resize', onResize);
    host?.removeEventListener('pointermove', onPointer);
    host?.removeEventListener('pointerleave', onLeave);
  };
}
