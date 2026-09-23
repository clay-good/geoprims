// Canvas 2D renderer for the map canvas (web/map-canvas): the Natural Earth
// base layer in the Atlas style, a graticule, and tool layers. This is the
// 2D-canvas path the spec requires as the fallback; colors come from the
// page's design tokens, so every display mode applies.
import { forward } from './projection.js';

const RAD = Math.PI / 180;
const MAX_LAT = 85.0511287798;
const mercY = (lat) => Math.log(Math.tan(Math.PI / 4 + (Math.max(-MAX_LAT, Math.min(MAX_LAT, lat)) * RAD) / 2));

/**
 * The design tokens the canvas draws with, resolved to concrete colors (night
 * mode's tokens are color-mix() expressions, which a canvas cannot parse).
 */
export function colors(el) {
  const probe = document.createElement('span');
  probe.style.display = 'none';
  (el.parentElement ?? el).append(probe);
  const resolve = (name) => {
    probe.style.color = `var(${name})`;
    return getComputedStyle(probe).color;
  };
  const out = {
    bg: resolve('--bg'), surface: resolve('--surface'), land: resolve('--land'), line: resolve('--line'), graticule: resolve('--graticule'),
    text: resolve('--text'), muted: resolve('--muted'), accent: resolve('--accent'), shadow: resolve('--shadow'),
    sans: getComputedStyle(el).getPropertyValue('--sans').trim(),
  };
  probe.remove();
  return out;
}

/** Mixes two rgb() colors, `k` of the way from a to b. */
function mix(a, b, k) {
  const pa = a.match(/[\d.]+/g).map(Number);
  const pb = b.match(/[\d.]+/g).map(Number);
  return `rgb(${[0, 1, 2].map((i) => Math.round(pa[i] + (pb[i] - pa[i]) * k)).join(',')})`;
}


/**
 * A pen that skips any vertex within a pixel of the last one it drew, so a
 * 100,000-vertex path costs what its pixels cost, not what its vertices do.
 * A pixel, not half: under strokes 1.5 to 3 px wide the difference cannot be
 * seen, and it cut a dense track's frame time by a fifth (the frames gate).
 * The first and last points of every run are always drawn.
 */
function pen(g) {
  let lx = NaN;
  let ly = NaN;
  let held = null;
  return {
    move(x, y) {
      this.flush();
      g.moveTo(x, y);
      [lx, ly] = [x, y];
    },
    line(x, y) {
      if (Math.abs(x - lx) < 1 && Math.abs(y - ly) < 1) {
        held = [x, y];
        return;
      }
      g.lineTo(x, y);
      [lx, ly] = [x, y];
      held = null;
    },
    flush() {
      if (held) g.lineTo(held[0], held[1]);
      held = null;
    },
  };
}

/**
 * A path's world coordinates, computed once and kept for as long as the path
 * is: longitudes made continuous, and each point's longitude, Mercator y, and
 * latitude in radians. Panning and zooming the flat map then only scales and
 * shifts these; nothing is projected again.
 */
const GEOMETRY = new WeakMap();
function geometry(points) {
  let geo = GEOMETRY.get(points);
  if (geo) return geo;
  const n = points.length;
  const lon = new Float64Array(n);
  const lat = new Float64Array(n);
  const M = new Float64Array(n);
  let lo = Infinity;
  let hi = -Infinity;
  for (let i = 0; i < n; i++) {
    const [l, p] = points[i];
    lon[i] = i === 0 ? l : lon[i - 1] + ((((l - lon[i - 1] + 180) % 360) + 360) % 360) - 180;
    lat[i] = p;
    M[i] = mercY(p);
    if (lon[i] < lo) lo = lon[i];
    if (lon[i] > hi) hi = lon[i];
  }
  geo = { n, lon, lat, M, lo, hi, lods: new Map() };
  GEOMETRY.set(points, geo);
  return geo;
}

/**
 * The sine and cosine of every vertex's latitude and longitude. The globe needs
 * all four for every vertex of every frame, and none of them change as the view
 * moves, so they are worth keeping; they are filled on first use because the
 * flat map never asks for them.
 */
function sphere(geo) {
  if (geo.sph) return geo.sph;
  const { n, lon, lat } = geo;
  const [slat, clat, slon, clon] = [new Float64Array(n), new Float64Array(n), new Float64Array(n), new Float64Array(n)];
  for (let i = 0; i < n; i++) {
    const [p, l] = [lat[i] * RAD, lon[i] * RAD];
    slat[i] = Math.sin(p);
    clat[i] = Math.cos(p);
    slon[i] = Math.sin(l);
    clon[i] = Math.cos(l);
  }
  geo.sph = { slat, clat, slon, clon };
  return geo.sph;
}

/**
 * The vertices worth drawing at this zoom: one level of detail per doubling of
 * scale, keeping a vertex only when it lies at least a pixel (at the
 * largest scale of its level) from the last one kept. The first and last
 * vertices are always kept, so a path starts and ends where it should.
 */
function detail(geo, mode, scale) {
  const level = Math.floor(Math.log2(Math.max(scale, 1e-9)));
  const key = `${mode}${level}`;
  let idx = geo.lods.get(key);
  if (idx) return idx;
  const tol = 1 / 2 ** (level + 1);
  const { n, lon, lat, M } = geo;
  const keep = [0];
  let k = 0;
  for (let i = 1; i < n; i++) {
    const dx = Math.abs(lon[i] - lon[k]) * RAD;
    const dy = mode === 'map' ? Math.abs(M[i] - M[k]) : Math.abs(lat[i] - lat[k]) * RAD;
    // On the globe and the polar view a degree of longitude shrinks with latitude.
    const sx = mode === 'map' || mode === 'equirect' ? dx : dx * Math.cos(lat[k] * RAD);
    if (sx >= tol || dy >= tol || i === n - 1) {
      keep.push(i);
      k = i;
    }
  }
  idx = Uint32Array.from(keep);
  geo.lods.set(key, idx);
  return idx;
}

/**
 * Traces a path. On the 2D map the path is drawn at -360°, 0, and +360°
 * offsets so shapes across the antimeridian stay contiguous (a copy wholly off
 * screen is skipped); on the globe, points on the far side break the path.
 * Only the vertices that can change a pixel are drawn.
 */
function trace(g, view, points, closed) {
  if (!points.length) return;
  const geo = geometry(points);
  const idx = detail(geo, view.mode, view.scale);
  const p = pen(g);
  if (view.mode === 'map' || view.mode === 'equirect') {
    const [cx, cy, s] = [view.width / 2, view.height / 2, view.scale];
    const Y = view.mode === 'map' ? geo.M : null;
    const y0 = view.mode === 'map' ? mercY(view.lat) : view.lat * RAD;
    for (const off of [-360, 0, 360]) {
      const x0 = cx + s * (geo.lo + off - view.lon) * RAD;
      const x1 = cx + s * (geo.hi + off - view.lon) * RAD;
      if (x1 < -2 || x0 > view.width + 2) continue;
      for (let j = 0; j < idx.length; j++) {
        const i = idx[j];
        const x = cx + s * (geo.lon[i] + off - view.lon) * RAD;
        const y = cy - s * ((Y ? Y[i] : geo.lat[i] * RAD) - y0);
        if (j === 0) p.move(x, y);
        else p.line(x, y);
      }
      p.flush();
      if (closed) g.closePath();
    }
    return;
  }
  if (view.mode === 'globe' && closed) {
    // Filled rings: skip those wholly on the far side; otherwise run hidden
    // stretches along the limb.
    const { slat, clat, slon, clon } = sphere(geo);
    const [sl0, cl0] = [Math.sin(view.lon * RAD), Math.cos(view.lon * RAD)];
    const [sp0, cp0] = [Math.sin(view.lat * RAD), Math.cos(view.lat * RAD)];
    const [cx, cy, s] = [view.width / 2, view.height / 2, view.scale];
    let any = false;
    for (let j = 0; j < idx.length && !any; j++) {
      const i = idx[j];
      any = sp0 * slat[i] + cp0 * clat[i] * (clon[i] * cl0 + slon[i] * sl0) >= 0;
    }
    if (!any) return;
    for (let j = 0; j < idx.length; j++) {
      const i = idx[j];
      const [sph, cph] = [slat[i], clat[i]];
      const [sdl, cdl] = [slon[i] * cl0 - clon[i] * sl0, clon[i] * cl0 + slon[i] * sl0];
      const x = cph * sdl;
      const y = cp0 * sph - sp0 * cph * cdl;
      // A hidden vertex is pinned to the limb: same direction, radius 1.
      const r = sp0 * sph + cp0 * cph * cdl >= 0 ? 1 : Math.hypot(x, y) || 1;
      if (j === 0) p.move(cx + (s * x) / r, cy - (s * y) / r);
      else p.line(cx + (s * x) / r, cy - (s * y) / r);
    }
    p.flush();
    g.closePath();
    return;
  }
  // The globe projects every vertex, so it is written out here rather than
  // called through forward(): the view's own sines and cosines are the same for
  // the whole path, each vertex's come from sphere(), and returning a pair of
  // numbers per vertex is an allocation per vertex at a hundred thousand of them.
  // The difference of the two longitudes is then had by angle addition alone.
  if (view.mode === 'globe') {
    const { slat, clat, slon, clon } = sphere(geo);
    const [sl0, cl0] = [Math.sin(view.lon * RAD), Math.cos(view.lon * RAD)];
    const [sp0, cp0] = [Math.sin(view.lat * RAD), Math.cos(view.lat * RAD)];
    const [cx, cy, s] = [view.width / 2, view.height / 2, view.scale];
    let open = false;
    for (let j = 0; j < idx.length; j++) {
      const i = idx[j];
      const [sph, cph] = [slat[i], clat[i]];
      const cdl = clon[i] * cl0 + slon[i] * sl0;
      if (sp0 * sph + cp0 * cph * cdl < 0) {
        // The far side of the globe: the path breaks here.
        if (open) p.flush();
        open = false;
        continue;
      }
      const x = cx + s * (cph * (slon[i] * cl0 - clon[i] * sl0));
      const y = cy - s * (cp0 * sph - sp0 * cph * cdl);
      if (open) p.line(x, y);
      else p.move(x, y);
      open = true;
    }
    p.flush();
    if (closed && open) g.closePath();
    return;
  }
  const pts = Array.from(idx, (i) => forward(view, geo.lon[i], geo.lat[i]));
  // The polar view: a ring that leaves the view is skipped whole (only
  // Antarctica, seen from the north).
  if (view.mode === 'polar' && closed) {
    if (pts.some((q) => !q)) return;
    pts.forEach(([x, y], j) => (j === 0 ? p.move(x, y) : p.line(x, y)));
    p.flush();
    g.closePath();
    return;
  }
  // Lines on the globe and the polar view break where they leave the view.
  let open = false;
  for (const q of pts) {
    if (!q) {
      if (open) p.flush();
      open = false;
      continue;
    }
    if (open) p.line(q[0], q[1]);
    else p.move(q[0], q[1]);
    open = true;
  }
  p.flush();
  if (closed && open) g.closePath();
}

const GRATICULES = new Map();
function graticule(step, reach = 80) {
  // Held by step and reach, of which the call site admits four pairs. The
  // lines themselves never move, and keeping the same arrays is what lets
  // geometry(), its levels of detail, and sphere() survive from frame to frame.
  const key = `${step}:${reach}`;
  const held = GRATICULES.get(key);
  if (held) return held;
  const lines = [];
  for (let lon = -180; lon < 180; lon += step) {
    const l = [];
    for (let lat = -reach; lat <= reach; lat += 2) l.push([lon, lat]);
    lines.push(l);
  }
  // Parallels on round numbers; the polar view shows them nearer the pole.
  const top = reach > 80 ? Math.floor(80 / step) * step : 60;
  for (let lat = -top; lat <= top; lat += step) {
    const l = [];
    for (let lon = -180; lon <= 180; lon += 2) l.push([lon, lat]);
    lines.push(l);
  }
  GRATICULES.set(key, lines);
  return lines;
}

/**
 * City labels: shown from each place's Natural Earth minimum zoom, largest
 * population first, skipping any that would overlap one already placed.
 */
function places(g, view, list, c) {
  // Web-map zoom level of the view: 256 px spans the world at zoom 0.
  const zoom = Math.log2((view.scale * 2 * Math.PI) / 256);
  const taken = [];
  g.font = `500 11px ${c.sans}`;
  let shown = 0;
  for (const p of list) {
    if (p.minZoom > zoom + 1 || shown >= 40) continue;
    const at = forward(view, p.lon, p.lat);
    if (!at || at[0] < 0 || at[1] < 0 || at[0] > view.width || at[1] > view.height) continue;
    const w = g.measureText(p.name).width;
    const box = [at[0] - 3, at[1] - 14, at[0] + w + 10, at[1] + 4];
    if (taken.some((b) => box[0] < b[2] && box[2] > b[0] && box[1] < b[3] && box[3] > b[1])) continue;
    taken.push(box);
    shown++;
    g.beginPath();
    g.arc(at[0], at[1], 2.2, 0, 2 * Math.PI);
    g.fillStyle = c.muted;
    g.fill();
    g.lineWidth = 3;
    g.strokeStyle = c.bg;
    g.strokeText(p.name, at[0] + 5, at[1] - 3);
    g.fillStyle = c.muted;
    g.fillText(p.name, at[0] + 5, at[1] - 3);
  }
}

/**
 * A path's direction and turns: a chevron at the middle of each leg long
 * enough to hold one, and a small dot at each turn point.
 */
function marks(g, view, layer, stroke, surface) {
  const pts = layer.points.map(([lon, lat]) => forward(view, lon, lat));
  for (let i = 0; i + 1 < pts.length; i++) {
    const [a, b] = [pts[i], pts[i + 1]];
    if (!a || !b) continue;
    const [dx, dy] = [b[0] - a[0], b[1] - a[1]];
    const len = Math.hypot(dx, dy);
    if (layer.arrows && len >= 36) {
      const [ux, uy] = [dx / len, dy / len];
      const [mx, my] = [(a[0] + b[0]) / 2 + ux * 3, (a[1] + b[1]) / 2 + uy * 3];
      g.beginPath();
      g.moveTo(mx - ux * 7 - uy * 5, my - uy * 7 + ux * 5);
      g.lineTo(mx, my);
      g.lineTo(mx - ux * 7 + uy * 5, my - uy * 7 - ux * 5);
      g.strokeStyle = surface;
      g.lineWidth = 5;
      g.stroke();
      g.strokeStyle = stroke;
      g.lineWidth = 2;
      g.stroke();
    }
  }
  if (!layer.stops) return;
  for (const p of pts.slice(1, -1)) {
    if (!p) continue;
    g.beginPath();
    g.arc(p[0], p[1], 2.5, 0, 2 * Math.PI);
    g.fillStyle = surface;
    g.fill();
    g.lineWidth = 1.5;
    g.strokeStyle = stroke;
    g.stroke();
  }
}

/** Draws the whole scene. `layers`: [{ kind: 'line'|'point'|'polygon', role: 'result'|'input'|'comparison', points, rings, label }]. */
/**
 * The globe's halo and lit sphere depend only on the canvas size, the globe's
 * radius, and the colors, so they are painted once into an offscreen canvas
 * and reused while the globe turns: two large gradients cost more than
 * everything else in a frame.
 */
let backdropCache = null;
function backdrop(width, height, r, c) {
  const dpr = globalThis.devicePixelRatio || 1;
  const key = `${width}x${height}@${dpr} r${r.toFixed(2)} ${c.surface} ${c.bg} ${c.shadow}`;
  if (backdropCache?.key === key) return backdropCache.canvas;
  const canvas = globalThis.OffscreenCanvas ? new OffscreenCanvas(Math.ceil(width * dpr), Math.ceil(height * dpr)) : Object.assign(document.createElement('canvas'), { width: Math.ceil(width * dpr), height: Math.ceil(height * dpr) });
  const g = canvas.getContext('2d');
  g.scale(dpr, dpr);
  // A soft halo, then the sphere, lit from the upper left.
  const [cx, cy] = [width / 2, height / 2];
  const halo = g.createRadialGradient(cx, cy, r * 0.98, cx, cy, r * 1.08);
  halo.addColorStop(0, mix(c.surface, c.shadow, 0.12));
  halo.addColorStop(1, c.surface);
  g.fillStyle = halo;
  g.beginPath();
  g.arc(cx, cy, r * 1.08, 0, 2 * Math.PI);
  g.fill();
  const sphere = g.createRadialGradient(cx - r * 0.35, cy - r * 0.35, r * 0.1, cx, cy, r);
  sphere.addColorStop(0, c.bg);
  sphere.addColorStop(1, mix(c.bg, c.shadow, 0.1));
  g.beginPath();
  g.arc(cx, cy, r, 0, 2 * Math.PI);
  g.fillStyle = sphere;
  g.fill();
  backdropCache = { key, canvas };
  return canvas;
}

export function draw(g, view, base, layers, c) {
  const { width, height } = view;
  g.clearRect(0, 0, width, height);
  g.lineJoin = 'round';
  g.lineCap = 'round';
  // Water is the page's background; on the globe, space around it is the surface.
  g.fillStyle = view.mode === 'globe' ? c.surface : c.bg;
  g.fillRect(0, 0, width, height);
  if (view.mode === 'globe') g.drawImage(backdrop(width, height, view.scale, c), 0, 0, width, height);
  // Graticule, faint.
  g.beginPath();
  for (const l of graticule(view.scale > width ? 10 : 30, view.mode === 'polar' ? 88 : 80)) trace(g, view, l, false);
  g.strokeStyle = c.graticule;
  g.lineWidth = 1;
  g.stroke();
  if (base) {
    g.beginPath();
    for (const ring of base.land) trace(g, view, ring, true);
    g.fillStyle = c.land;
    g.fill('evenodd');
    g.strokeStyle = c.line;
    g.lineWidth = 0.75;
    g.stroke();
    g.beginPath();
    for (const ring of base.lakes) trace(g, view, ring, true);
    g.fillStyle = c.bg;
    g.fill();
    // State and province lines, fainter than the national borders.
    if (base.states?.length) {
      g.beginPath();
      for (const l of base.states) trace(g, view, l, false);
      g.strokeStyle = mix(c.line, c.muted, 0.25);
      g.lineWidth = 0.7;
      g.setLineDash([4, 3]);
      g.stroke();
      g.setLineDash([]);
    }
    g.beginPath();
    for (const l of base.borders) trace(g, view, l, false);
    g.strokeStyle = mix(c.line, c.muted, 0.35);
    g.lineWidth = 0.8;
    g.stroke();
    if (base.places?.length) places(g, view, base.places, c);
  }
  if (view.mode === 'globe') {
    g.beginPath();
    g.arc(width / 2, height / 2, view.scale, 0, 2 * Math.PI);
    g.strokeStyle = c.line;
    g.lineWidth = 1;
    g.stroke();
  }
  // Tool layers: inputs and comparisons in neutrals, the result in the accent,
  // each over a light casing so it reads over land and water alike.
  const order = { comparison: 0, input: 1, result: 2 };
  for (const layer of [...layers].sort((a, b) => order[a.role] - order[b.role])) {
    const stroke = layer.role === 'result' ? c.accent : c.muted;
    if (layer.kind === 'polygon' && layer.cell) {
      // A grid cell: the origin strong, the rest fading with grid distance.
      // A compacted cell stands for a group of finer ones, so it is drawn
      // dashed: the ground is right, the edges inside it are not being shown.
      const origin = layer.role === 'result';
      const w = origin ? 1 : (layer.weight ?? 0.8);
      g.beginPath();
      for (const ring of layer.rings) trace(g, view, ring, true);
      g.fillStyle = c.accent;
      g.globalAlpha = origin ? 0.28 : 0.1 * w;
      g.fill();
      g.globalAlpha = 0.3 + 0.7 * w;
      g.strokeStyle = c.accent;
      g.lineWidth = origin ? 3 : 1.5;
      if (layer.compacted) g.setLineDash([5, 3]);
      g.stroke();
      g.setLineDash([]);
      g.globalAlpha = 1;
    } else if (layer.kind === 'polygon') {
      g.beginPath();
      for (const ring of layer.rings) trace(g, view, ring, true);
      g.fillStyle = stroke;
      g.globalAlpha = 0.14;
      g.fill('evenodd');
      g.globalAlpha = 1;
      g.strokeStyle = c.surface;
      g.lineWidth = 4;
      g.stroke();
      g.strokeStyle = stroke;
      g.lineWidth = 2;
      g.stroke();
    } else if (layer.kind === 'line') {
      g.beginPath();
      trace(g, view, layer.points, false);
      g.setLineDash([]);
      g.strokeStyle = c.surface;
      g.lineWidth = layer.role === 'result' ? 8 : 4;
      g.stroke();
      g.setLineDash(layer.role === 'comparison' ? [6, 5] : []);
      g.strokeStyle = stroke;
      g.lineWidth = layer.role === 'result' ? 3 : 1.5;
      g.stroke();
      g.setLineDash([]);
      if (layer.arrows || layer.stops) marks(g, view, layer, stroke, c.surface);
    } else if (layer.kind === 'point') {
      for (const [lon, lat] of layer.points) {
        // forward() places a point at its copy nearest the view center.
        const p = forward(view, lon, lat);
        if (!p) continue;
        // Detail points are many — a mission's camera triggers — so they are
        // drawn small enough not to bury the path they sit on.
        const r = layer.role === 'result' ? 7 : layer.role === 'detail' ? 2.5 : 5.5;
        if (layer.role !== 'detail') {
          g.beginPath();
          g.arc(p[0], p[1], r + 2.5, 0, 2 * Math.PI);
          g.fillStyle = c.surface;
          g.fill();
        }
        g.beginPath();
        g.arc(p[0], p[1], r, 0, 2 * Math.PI);
        g.fillStyle = stroke;
        g.fill();
        if (layer.role === 'result') {
          g.beginPath();
          g.arc(p[0], p[1], r * 0.4, 0, 2 * Math.PI);
          g.fillStyle = c.surface;
          g.fill();
        }
        if (layer.label) {
          // The label on a small pill, above and to the right of the marker.
          g.font = `600 12px ${c.sans}`;
          const w = g.measureText(layer.label).width + 12;
          const [lx, ly] = [p[0] + r + 4, p[1] - r - 20];
          g.beginPath();
          g.roundRect(lx, ly, w, 20, 10);
          g.fillStyle = c.surface;
          g.fill();
          g.strokeStyle = c.line;
          g.lineWidth = 1;
          g.stroke();
          g.fillStyle = c.text;
          g.fillText(layer.label, lx + 6, ly + 14);
        }
      }
    }
  }
}
