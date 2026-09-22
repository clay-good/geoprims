// Map projections for the canvas (web/map-canvas "Canvas modes"): Web
// Mercator (`map`), equirectangular (`equirect`), and a polar azimuthal
// equidistant view (`polar`, centered on the pole of the hemisphere the view's
// latitude is in, with its central meridian pointing down in the north and up
// in the south) for the 2D map, and an orthographic globe. A view is
// { mode, lon, lat, scale, width, height }: the center, pixels per radian,
// and the canvas size. forward() returns screen [x, y] or null when the point
// is not visible (the far side of the globe); inverse() returns [lon, lat] or
// null off the map.
const RAD = Math.PI / 180;
export const MAX_MERCATOR_LAT = 85.0511287798;
/** The polar view reaches this far from its pole, in degrees of colatitude. */
export const POLAR_REACH = 150;
/** Readout names for each mode. */
export const PROJECTION_NAMES = { map: 'Web Mercator', equirect: 'Equirectangular', polar: 'Polar azimuthal equidistant', globe: 'Globe' };
const hemi = (view) => (view.lat >= 0 ? 1 : -1);

const wrap = (lon) => ((((lon + 180) % 360) + 360) % 360) - 180;
const mercY = (lat) => Math.log(Math.tan(Math.PI / 4 + (Math.max(-MAX_MERCATOR_LAT, Math.min(MAX_MERCATOR_LAT, lat)) * RAD) / 2));

export function forward(view, lon, lat) {
  if (view.mode === 'globe') {
    const [l0, p0] = [view.lon * RAD, view.lat * RAD];
    const [l, p] = [lon * RAD, lat * RAD];
    const cosc = Math.sin(p0) * Math.sin(p) + Math.cos(p0) * Math.cos(p) * Math.cos(l - l0);
    if (cosc < 0) return null;
    const x = Math.cos(p) * Math.sin(l - l0);
    const y = Math.cos(p0) * Math.sin(p) - Math.sin(p0) * Math.cos(p) * Math.cos(l - l0);
    return [view.width / 2 + view.scale * x, view.height / 2 - view.scale * y];
  }
  if (view.mode === 'polar') {
    const h = hemi(view);
    const colat = 90 - h * lat;
    if (colat > POLAR_REACH) return null;
    const rho = colat * RAD * view.scale;
    const dl = (lon - view.lon) * RAD;
    return [view.width / 2 + rho * Math.sin(dl), view.height / 2 + h * rho * Math.cos(dl)];
  }
  const x = wrap(lon - view.lon) * RAD;
  const y = view.mode === 'equirect' ? (lat - view.lat) * RAD : mercY(lat) - mercY(view.lat);
  return [view.width / 2 + view.scale * x, view.height / 2 - view.scale * y];
}

/**
 * Like forward(), but a point on the far side of the globe lands on the limb
 * in its direction, so a filled ring that crosses the edge follows the edge
 * instead of cutting a chord across the disk.
 */
export function forwardLimb(view, lon, lat) {
  const p = forward(view, lon, lat);
  if (p || view.mode !== 'globe') return p;
  const [l0, p0] = [view.lon * RAD, view.lat * RAD];
  const [l, q] = [lon * RAD, lat * RAD];
  const x = Math.cos(q) * Math.sin(l - l0);
  const y = Math.cos(p0) * Math.sin(q) - Math.sin(p0) * Math.cos(q) * Math.cos(l - l0);
  const r = Math.hypot(x, y) || 1;
  return [view.width / 2 + (view.scale * x) / r, view.height / 2 - (view.scale * y) / r];
}

export function inverse(view, sx, sy) {
  const x = (sx - view.width / 2) / view.scale;
  const y = (view.height / 2 - sy) / view.scale;
  if (view.mode === 'globe') {
    const rho = Math.hypot(x, y);
    if (rho > 1) return null;
    const c = Math.asin(rho);
    const [l0, p0] = [view.lon * RAD, view.lat * RAD];
    const lat = rho === 0 ? view.lat : Math.asin(Math.cos(c) * Math.sin(p0) + (y * Math.sin(c) * Math.cos(p0)) / rho) / RAD;
    const lon = l0 + Math.atan2(x * Math.sin(c), rho * Math.cos(c) * Math.cos(p0) - y * Math.sin(c) * Math.sin(p0));
    return [wrap(lon / RAD), lat];
  }
  if (view.mode === 'polar') {
    const h = hemi(view);
    const rho = Math.hypot(x, y);
    if (rho / RAD > POLAR_REACH) return null;
    return [wrap(view.lon + Math.atan2(x, -h * y) / RAD), h * (90 - rho / RAD)];
  }
  if (view.mode === 'equirect') {
    const lat = view.lat + y / RAD;
    return Math.abs(lat) > 90 ? null : [wrap(view.lon + x / RAD), lat];
  }
  const lat = (2 * Math.atan(Math.exp(y + mercY(view.lat))) - Math.PI / 2) / RAD;
  return [wrap(view.lon + x / RAD), lat];
}

/**
 * A view that frames the given [lon, lat] points with some margin. Longitudes
 * are unwrapped around the first point, so a set across the antimeridian is
 * framed as one group rather than the whole world.
 */
export function frame(mode, points, width, height) {
  if (!points.length) return { mode, lon: 0, lat: 20, scale: Math.min(width, height) / (mode === 'globe' ? 2.2 : 6.5), width, height };
  const base = points[0][0];
  const lons = points.map(([lon]) => base + wrap(lon - base));
  const lats = points.map(([, lat]) => lat);
  const lon = wrap((Math.min(...lons) + Math.max(...lons)) / 2);
  const lat = (Math.min(...lats) + Math.max(...lats)) / 2;
  if (mode === 'globe') {
    // Scale so the farthest point is within 80% of the globe's radius.
    const view = { mode, lon, lat, scale: 1, width, height };
    const far = Math.max(1e-6, ...points.map(([pl, pp]) => {
      const p = forward({ ...view, scale: 1, width: 0, height: 0 }, pl, pp);
      return p ? Math.hypot(p[0], p[1]) : 1;
    }));
    // A point or a small area still shows a good part of the hemisphere around it.
    const MIN_FAR = 0.5;
    return { ...view, scale: Math.min(width, height) * 0.45 / Math.max(far, MIN_FAR) };
  }
  if (mode === 'polar') {
    // Centered on the pole of the hemisphere most of the points are in, with
    // the points' middle meridian toward the viewer.
    const h = lats.reduce((a, b) => a + b, 0) >= 0 ? 1 : -1;
    const reach = Math.max(10, ...lats.map((p) => 90 - h * p)) * RAD;
    return { mode, lon, lat: h * 90, scale: (Math.min(width, height) * 0.45) / Math.min(reach, POLAR_REACH * RAD), width, height };
  }
  // At least about 12° across, so a single point sits among coastlines and borders.
  const MIN_SPAN = 12 * RAD;
  const spanX = Math.max(MIN_SPAN, (Math.max(...lons) - Math.min(...lons)) * RAD);
  const ys = (p) => (mode === 'equirect' ? p * RAD : mercY(p));
  const spanY = Math.max(MIN_SPAN * 0.6, ys(Math.max(...lats)) - ys(Math.min(...lats)));
  const scale = Math.min((width * 0.6) / spanX, (height * 0.6) / spanY, width * 2000);
  return { mode, lon, lat, scale: Math.max(scale, width / (2 * Math.PI)), width, height };
}

/** Longitudes made continuous along a path (no ±360° jumps), for drawing across the antimeridian. */
export function unwrap(points) {
  const out = [];
  for (const [lon, lat] of points) {
    if (!out.length) out.push([lon, lat]);
    else {
      const prev = out[out.length - 1][0];
      out.push([prev + wrap(lon - prev), lat]);
    }
  }
  return out;
}

/** Decodes a Natural Earth ring: hundredths of a degree, delta-encoded after the first pair. */
export function decode(flat) {
  const out = [];
  let lon = 0;
  let lat = 0;
  for (let i = 0; i < flat.length; i += 2) {
    lon = i === 0 ? flat[0] : lon + flat[i];
    lat = i === 0 ? flat[1] : lat + flat[i + 1];
    out.push([lon / 100, lat / 100]);
  }
  return out;
}
