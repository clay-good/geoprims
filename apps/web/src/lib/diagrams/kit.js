// Shared pieces of the vector diagrams (web/map-canvas "Canvas modes"): the
// SVG frame and marks, unit tables, and result readers every domain uses.
// Split out of diagrams.js so a page loads only its own domain's drawings.

export const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);
export const R = Math.PI / 180;

/** Speeds and lengths in base units (m/s, m) from "120 kt", "10 m/s", or a bare number in `unit`. */
export const SPEED = { kt: 1852 / 3600, kts: 1852 / 3600, knots: 1852 / 3600, mph: 0.44704, 'km/h': 1 / 3.6, kph: 1 / 3.6, 'm/s': 1, 'ft/s': 0.3048 };
export const LENGTH = { m: 1, km: 1000, nm: 1852, ft: 0.3048, mi: 1609.344, sm: 1609.344 };
export function measure(v, table, unit) {
  if (typeof v === 'number') return v * table[unit];
  const m = /^\s*([-+]?[\d.]+(?:e[-+]?\d+)?)\s*([a-z/][a-z0-9/]*)?\s*$/i.exec(String(v ?? ''));
  if (!m) return null;
  const k = table[(m[2] || unit).toLowerCase()];
  return k ? Number(m[1]) * k : null;
}
export const deg = (v) => {
  const n = Number.parseFloat(String(v ?? '').replace(/[°º]/g, ' '));
  return Number.isFinite(n) ? n : null;
};
export const val = (result, k) => {
  const v = result.result?.[k];
  return typeof v === 'number' ? v : v?.value;
};
export const disp = (result, k) => result.display?.[k] ?? '';
export const unitOf = (result, k) => result.result?.[k]?.unit ?? '';

/** Compass vector: bearing (deg true) and length → SVG dx, dy (y down). */
export const vec = (bearing, len) => [len * Math.sin(bearing * R), -len * Math.cos(bearing * R)];

/**
 * The scope the marker ids belong to. A page may draw the same diagram twice —
 * compact under the answer and full-size in the canvas — and two elements may
 * not share an id, so each drawing sets this before it builds its markup.
 */
export let scope = '';

export const headId = (cls) => `dg-head-${cls.includes('accent') ? 'accent' : 'muted'}${scope}`;

export function arrow(x1, y1, x2, y2, cls, label, labelAt = 0.55, side = -1) {
  // The label sits beside the vector, `side` -1 to its left and 1 to its right.
  const len = Math.hypot(x2 - x1, y2 - y1) || 1;
  const [nx, ny] = [(-(y2 - y1) / len) * side, ((x2 - x1) / len) * side];
  const [lx, ly] = [x1 + (x2 - x1) * labelAt + nx * 12, y1 + (y2 - y1) * labelAt + ny * 12];
  const anchor = Math.abs(nx) > Math.abs(ny) ? (nx > 0 ? 'start' : 'end') : 'middle';
  const t = label ? `<text class="dg-label" text-anchor="${anchor}" x="${lx.toFixed(1)}" y="${(ly + (ny > 0.5 ? 10 : ny < -0.5 ? -2 : 4)).toFixed(1)}">${esc(label)}</text>` : '';
  return `<line class="dg-casing" x1="${x1.toFixed(1)}" y1="${y1.toFixed(1)}" x2="${x2.toFixed(1)}" y2="${y2.toFixed(1)}"/><line class="${cls}" x1="${x1.toFixed(1)}" y1="${y1.toFixed(1)}" x2="${x2.toFixed(1)}" y2="${y2.toFixed(1)}" marker-end="url(#${headId(cls)})"/>${t}`;
}

/** Scales and centers points (in any units, y up) into the 320 × 240 drawing, returning a mapper. */
export function fit(points, w = 220, h = 150) {
  const xs = points.map((p) => p[0]);
  const ys = points.map((p) => p[1]);
  const k = Math.min(w / Math.max(Math.max(...xs) - Math.min(...xs), 1e-9), h / Math.max(Math.max(...ys) - Math.min(...ys), 1e-9));
  const [mx, my] = [(Math.max(...xs) + Math.min(...xs)) / 2, (Math.max(...ys) + Math.min(...ys)) / 2];
  return ([x, y]) => [160 + (x - mx) * k, 128 - (y - my) * k];
}

export function svg(body, title) {
  return `<svg viewBox="0 0 320 240" class="dg" role="img" aria-label="${esc(title)}" xmlns="http://www.w3.org/2000/svg"><defs>` +
    `<marker id="dg-head-accent${scope}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path class="dg-head-accent" d="M0 0L10 5L0 10z"/></marker>` +
    `<marker id="dg-head-muted${scope}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path class="dg-head-muted" d="M0 0L10 5L0 10z"/></marker>` +
    `</defs>${body}</svg>`;
}

export const f1 = (n) => n.toFixed(1);
export const line = (cls, a, b) => `<line class="${cls}" x1="${f1(a[0])}" y1="${f1(a[1])}" x2="${f1(b[0])}" y2="${f1(b[1])}"/>`;
export const text = (cls, x, y, t, anchor = 'start') => `<text class="${cls}" text-anchor="${anchor}" x="${f1(x)}" y="${f1(y)}">${esc(t)}</text>`;
export const dot = (x, y, cls = 'dg-dot') => `<circle class="${cls}" cx="${f1(x)}" cy="${f1(y)}" r="4.5"/>`;

/** A station like "7+00.00" as a number (700). */
export const station = (s) => {
  const m = /^\s*(-?\d+)\+(\d+(?:\.\d+)?)\s*$/.exec(String(s ?? ''));
  return m ? Number(m[1]) * 100 + Math.sign(Number(m[1]) || 1) * Number(m[2]) : Number.parseFloat(s);
};

/** An input as typed, with its default unit when it is a bare number. */
export const typed = (v, unit) => (typeof v === 'number' || /^\s*[-+]?[\d.]+\s*$/.test(String(v ?? '')) ? `${v} ${unit}` : String(v ?? ''));
export const path = (pts, close = false) => pts.map((p, i) => `${i ? 'L' : 'M'}${f1(p[0])} ${f1(p[1])}`).join('') + (close ? 'Z' : '');

export const kt = (v) => measure(v, SPEED, 'kt');
/** Plane survey points (northing, easting) in one length unit: [easting, northing] in meters. */
export const plane = (n, e, unit) => [measure(e, LENGTH, unit), measure(n, LENGTH, unit)];

/** The round factor (1, 2, or 5 × 10^n) that draws `len` drawing units at most `target` long; 1 when it needs none. */
export function exaggeration(len, target = 50) {
  if (!(len > 0) || len >= target) return 1;
  const p = 10 ** Math.floor(Math.log10(target / len));
  return [5, 2, 1].map((m) => m * p).find((f) => len * f <= target) ?? p;
}

/** Sets the marker-id scope for the drawing being built (see `scope`). */
export function setScope(s) {
  scope = s;
}

/** Draws one diagram with its own marker ids, or null when it cannot. */
export function draw(f, args, result, at = '', view = {}) {
  if (!f || !result?.ok) return null;
  scope = at ? `-${at}` : '';
  try {
    return f(args, result, view);
  } catch {
    return null;
  } finally {
    scope = '';
  }
}
