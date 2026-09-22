// Canvas input (web/map-canvas, "Canvas input"): the input points a reader
// can drag, which point a press lands on, and how many decimals a dragged
// coordinate earns. The form stays the keyboard path for every one of them.
import { forward } from './projection.js';
import { wrapLon } from './readout.js';

/** The draggable points: input points that name the fields they came from. */
export const handlesOf = (layers) =>
  layers.filter((l) => l.kind === 'point' && l.role === 'input' && l.field).map((l) => ({ field: l.field, label: l.label, lon: l.points[0][0], lat: l.points[0][1] }));

/** The handle under a press at (x, y), within `radius` px, nearest first; or null. */
export function handleAt(handles, view, x, y, radius = 18) {
  let best = null;
  let bestD = radius;
  for (const h of handles) {
    const p = forward(view, h.lon, h.lat);
    if (!p) continue;
    const d = Math.hypot(p[0] - x, p[1] - y);
    if (d <= bestD) [best, bestD] = [h, d];
  }
  return best;
}

/**
 * A dragged coordinate to the decimals one screen pixel resolves (about
 * 111 km a degree), from 3 to 7, so a drag never invents millimeters.
 */
export function dragDegrees(lat, lon, metersPerPixel) {
  const decimals = Math.max(3, Math.min(7, Math.ceil(-Math.log10(Math.max(metersPerPixel, 1e-3) / 111_320))));
  const f = (v) => `${Number(v.toFixed(decimals))}`;
  return { lat: f(Math.max(-90, Math.min(90, lat))), lon: f(wrapLon(lon)) };
}

/** The point a click sets: the tool's single input point, when it has exactly one field pair. */
export const clickTarget = (tool) => (tool.inputs.properties.lat && tool.inputs.properties.lon && !tool.inputs.properties.lat1 ? { lat: 'lat', lon: 'lon' } : null);
