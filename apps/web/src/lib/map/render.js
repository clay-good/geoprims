// Canvas 2D renderer for the map canvas (web/map-canvas): the Natural Earth
// base layer in the Atlas style, a graticule, and tool layers. This is the
// 2D-canvas path the spec requires as the fallback; colors come from the
// page's design tokens, so every display mode applies.
import { forward, forwardLimb, unwrap } from './projection.js';

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

/** Screen x, y on the 2D map without wrapping longitude, for unwrapped paths. */
function mapXY(view, lon, lat) {
  return [view.width / 2 + view.scale * (lon - view.lon) * RAD, view.height / 2 - view.scale * (mercY(lat) - mercY(view.lat))];
}

/**
 * Traces a path. On the 2D map the path is unwrapped and drawn at -360°, 0,
 * and +360° offsets so shapes across the antimeridian stay contiguous; on the
 * globe, points on the far side break the path.
 */
function trace(g, view, points, closed) {
  if (view.mode === 'globe' && closed) {
    // Filled rings: skip those wholly on the far side; otherwise run hidden
    // stretches along the limb.
    if (!points.some(([lon, lat]) => forward(view, lon, lat))) return;
    points.forEach(([lon, lat], i) => {
      const [x, y] = forwardLimb(view, lon, lat);
      if (i === 0) g.moveTo(x, y);
      else g.lineTo(x, y);
    });
    g.closePath();
    return;
  }
  if (view.mode === 'globe') {
    let open = false;
    for (const [lon, lat] of points) {
      const p = forward(view, lon, lat);
      if (!p) {
        open = false;
        continue;
      }
      if (open) g.lineTo(p[0], p[1]);
      else g.moveTo(p[0], p[1]);
      open = true;
    }
    if (closed && open) g.closePath();
    return;
  }
  const pts = unwrap(points);
  for (const off of [-360, 0, 360]) {
    pts.forEach(([lon, lat], i) => {
      const [x, y] = mapXY(view, lon + off, lat);
      if (i === 0) g.moveTo(x, y);
      else g.lineTo(x, y);
    });
    if (closed) g.closePath();
  }
}

function graticule(step) {
  const lines = [];
  for (let lon = -180; lon < 180; lon += step) {
    const l = [];
    for (let lat = -80; lat <= 80; lat += 2) l.push([lon, lat]);
    lines.push(l);
  }
  for (let lat = -60; lat <= 60; lat += step) {
    const l = [];
    for (let lon = -180; lon <= 180; lon += 2) l.push([lon, lat]);
    lines.push(l);
  }
  return lines;
}

/** Draws the whole scene. `layers`: [{ kind: 'line'|'point'|'polygon', role: 'result'|'input'|'comparison', points, rings, label }]. */
export function draw(g, view, base, layers, c) {
  const { width, height } = view;
  g.clearRect(0, 0, width, height);
  g.lineJoin = 'round';
  g.lineCap = 'round';
  // Water is the page's background; on the globe, space around it is the surface.
  g.fillStyle = view.mode === 'globe' ? c.surface : c.bg;
  g.fillRect(0, 0, width, height);
  if (view.mode === 'globe') {
    // A soft halo, then the sphere, lit from the upper left.
    const [cx, cy, r] = [width / 2, height / 2, view.scale];
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
  }
  // Graticule, faint.
  g.beginPath();
  for (const l of graticule(view.scale > width ? 10 : 30)) trace(g, view, l, false);
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
    g.beginPath();
    for (const l of base.borders) trace(g, view, l, false);
    g.strokeStyle = c.line;
    g.lineWidth = 0.5;
    g.stroke();
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
    if (layer.kind === 'polygon') {
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
    } else if (layer.kind === 'point') {
      for (const [lon, lat] of layer.points) {
        // forward() places a point at its copy nearest the view center.
        const p = forward(view, lon, lat);
        if (!p) continue;
        const r = layer.role === 'result' ? 7 : 5.5;
        g.beginPath();
        g.arc(p[0], p[1], r + 2.5, 0, 2 * Math.PI);
        g.fillStyle = c.surface;
        g.fill();
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
