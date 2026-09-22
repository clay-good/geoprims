// The cursor readout (web/map-canvas, "Measurement readouts"): the point under
// the pointer in the reader's chosen coordinate format. Degrees are written
// here; MGRS and UTM come from the core's own grid tools, so the readout and
// the tools can never disagree about a grid reference.

const pad = (n, width) => String(n).padStart(width, '0');

/** One angle as degrees, degrees and minutes, or degrees, minutes, and seconds, with its hemisphere. */
function angle(v, [pos, neg], fmt) {
  const h = v < 0 ? neg : pos;
  const a = Math.abs(v);
  if (fmt === 'ddm') {
    // Round once at the last digit shown, then split, so 59.9995′ never prints as 60′.
    const thousandths = Math.round(a * 60000);
    const d = Math.floor(thousandths / 60000);
    const m = (thousandths - d * 60000) / 1000;
    return `${d}° ${m.toFixed(3).padStart(6, '0')}′ ${h}`;
  }
  const tenths = Math.round(a * 36000);
  const d = Math.floor(tenths / 36000);
  const m = Math.floor((tenths - d * 36000) / 600);
  const s = (tenths - d * 36000 - m * 600) / 10;
  return `${d}° ${pad(m, 2)}′ ${s.toFixed(1).padStart(4, '0')}″ ${h}`;
}

/** Latitude and longitude in a degree format: dd (signed decimal), ddm, or dms. */
export function formatDegrees(lat, lon, fmt = 'dd') {
  if (fmt === 'ddm' || fmt === 'dms') return `${angle(lat, ['N', 'S'], fmt)}, ${angle(lon, ['E', 'W'], fmt)}`;
  return `${lat.toFixed(4)}°, ${lon.toFixed(4)}°`;
}

/** The coarsest MGRS precision that still resolves one screen pixel. */
export function mgrsPrecision(metersPerPixel) {
  return ['1m', '10m', '100m', '1km', '10km'].find((p) => (p.endsWith('km') ? 1000 : 1) * parseFloat(p) >= metersPerPixel) ?? '100km';
}

/** Longitude folded into [-180, 180), for a pointer over a panned map. */
export const wrapLon = (lon) => ((((lon + 180) % 360) + 360) % 360) - 180;

/**
 * The readout text for a point. Grid formats ask the core through `invoke`
 * and fall back to degrees, saying why, where the grid does not reach.
 */
export async function readoutText(lat, lon, fmt, { invoke, metersPerPixel = 1 } = {}) {
  lon = wrapLon(lon);
  if (fmt === 'mgrs' && invoke) {
    const out = await invoke('geodesy.grid-ref.mgrs-forward', { lat, lon, precision: mgrsPrecision(metersPerPixel) });
    if (out === null) return null; // superseded by a newer pointer position
    return out.ok ? out.result.mgrs_spaced : `${formatDegrees(lat, lon)} (outside MGRS)`;
  }
  if (fmt === 'utm' && invoke) {
    const out = await invoke('geodesy.utm.forward', { lat, lon });
    if (out === null) return null;
    if (!out.ok) return `${formatDegrees(lat, lon)} (outside UTM)`;
    const r = out.result;
    return `${r.zone}${r.hemisphere} ${Math.round(r.easting.value)} mE ${Math.round(r.northing.value)} mN`;
  }
  return formatDegrees(lat, lon, fmt);
}

/**
 * The magnetic north indicator's angle in degrees clockwise from true north,
 * when the tool works in magnetic values: its declination output (east positive).
 */
export function magneticNorth(result) {
  const d = result?.ok ? result.result?.declination : null;
  const v = typeof d === 'object' && d !== null ? d.value : d;
  return typeof v === 'number' && Number.isFinite(v) ? v : null;
}
