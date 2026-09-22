// Coordinates that arrive in a projected system (web/io-formats, "Coordinate
// order and CRS safety"): a CSV the reader says is UTM or State Plane, or a
// legacy GeoJSON `crs` member naming a WGS 84 UTM zone. Each is turned into
// latitude and longitude by the core's own inverse tools, the same ones the
// UTM and State Plane pages run, one batch call for the whole file. Anything
// else that is not WGS 84 is refused rather than guessed at.

/** The systems a CSV's two columns can be in. */
export const CSV_CRS = [
  { id: 'wgs84', label: 'Latitude and longitude (WGS 84)' },
  { id: 'utm', label: 'UTM (WGS 84), easting and northing' },
  { id: 'spcs', label: 'State Plane (NAD83), easting and northing' },
];

/** State Plane distance units, as the core names them. */
export const SPCS_UNITS = [
  ['legal', 'The zone’s legal unit'],
  ['m', 'Meters'],
  ['ftUS', 'US survey feet'],
  ['ft', 'International feet'],
];

/**
 * A GeoJSON `crs` name as a system this reader handles: WGS 84 itself, or a
 * WGS 84 UTM zone (EPSG 326zz north, 327zz south). Null for anything else.
 */
export function crsFromName(name) {
  const s = String(name ?? '');
  if (/CRS84|(^|\D)4326$/i.test(s) || /EPSG::?4326\b/i.test(s)) return { kind: 'wgs84' };
  const m = /EPSG:{1,2}(32[67])(\d\d)$/i.exec(s.trim());
  if (m) {
    const zone = Number(m[2]);
    if (zone >= 1 && zone <= 60) return { kind: 'utm', zone, hemisphere: m[1] === '326' ? 'N' : 'S', name: s };
  }
  return null;
}

/** What a projected system is called in a note to the reader. */
export function crsLabel(crs) {
  if (crs.kind === 'utm') return `UTM zone ${crs.zone}${crs.hemisphere} (WGS 84)`;
  if (crs.kind === 'spcs') return `State Plane zone ${crs.zone} (NAD83)`;
  return 'WGS 84';
}

/**
 * What changing datum means for the numbers, said once: State Plane is NAD83,
 * and its latitudes and longitudes are used as they come.
 */
export const datumNote = (crs) =>
  crs.kind === 'spcs' ? 'State Plane is NAD83: the latitudes and longitudes are NAD83, which sits within about 2 m of WGS 84 in the lower 48.' : '';

/** The core call that inverts one [easting, northing] pair in `crs`. */
export function inverseCall(crs) {
  if (crs.kind === 'utm') {
    return { id: 'geodesy.utm.inverse', input: ([x, y]) => ({ zone: crs.zone, hemisphere: crs.hemisphere, easting: `${x} m`, northing: `${y} m` }) };
  }
  if (crs.kind === 'spcs') {
    return { id: 'geodesy.spcs.spcs83-inverse', input: ([x, y]) => ({ zone: String(crs.zone), easting: String(x), northing: String(y), unit: crs.unit ?? 'legal' }) };
  }
  throw new Error(`No inverse for ${crs.kind}.`);
}

/**
 * [easting, northing] pairs to [lon, lat] through the core, in one batch.
 * Returns the pairs, or the first pair the core refused with its message.
 */
export async function toGeographic(crs, pairs, invokeBatch) {
  if (!pairs.length) return { ok: true, coords: [] };
  const { id, input } = inverseCall(crs);
  const raw = await invokeBatch(id, JSON.stringify(pairs.map(input)));
  const out = typeof raw === 'string' ? JSON.parse(raw) : raw;
  if (!Array.isArray(out)) return { ok: false, index: 0, message: out?.error?.message ?? 'Those coordinates could not be converted.' };
  const coords = [];
  for (const [i, r] of out.entries()) {
    if (!r.ok) return { ok: false, index: i, message: r.error?.message ?? 'That coordinate could not be converted.' };
    coords.push([r.result.lon.value, r.result.lat.value]);
  }
  return { ok: true, coords };
}

/**
 * A parsed file whose geometries are in a projected `crs`, with every
 * coordinate converted to [lon, lat] and the conversion reported.
 */
export async function projectParsed(parsed, invokeBatch) {
  const crs = parsed?.crs;
  if (!parsed?.ok || !crs || crs.kind === 'wgs84') return parsed;
  const flat = parsed.geometries.flatMap((g) => g.coordinates);
  const done = await toGeographic(crs, flat, invokeBatch);
  if (!done.ok) return { ...parsed, ok: false, message: `A coordinate in ${crsLabel(crs)} could not be converted: ${done.message}` };
  let k = 0;
  const geometries = parsed.geometries.map((g) => ({ ...g, coordinates: g.coordinates.map(() => done.coords[k++]) }));
  return { ...parsed, geometries, crs: { kind: 'wgs84' }, converted: crsLabel(crs) };
}
