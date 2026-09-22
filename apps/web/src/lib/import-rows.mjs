// From a file a reader brought to the rows of a tool's list input
// (web/io-formats: "Drop a GPX track"). The readers in import.mjs give
// geometries as [lon, lat]; a list input takes rows in its own columns. This
// is the one place the two meet, so the order of a pair is decided here and
// nowhere else.

import { crsLabel, datumNote, toGeographic } from './crs.mjs';

const num = (x) => Number(x.toFixed(9));

/**
 * Rows for a list input from a parsed file, or a reason it cannot fill it.
 * Polygon inputs (a `ring` column) take every ring, the outer one as ring 0
 * and holes after it, without the repeated closing corner the inputs do not
 * need. Line inputs take the first line or ring. Point-only files fill a line
 * input in the order the points appear.
 */
export function rowsFor(schema, parsed) {
  if (!parsed?.ok) return { ok: false, message: parsed?.message ?? 'That file could not be read.' };
  const columns = Object.keys(schema?.items?.properties ?? {});
  if (!columns.includes('lat') || !columns.includes('lon')) {
    return { ok: false, message: 'This input does not take latitude and longitude.' };
  }
  const max = schema.maxItems ?? Infinity;
  const geoms = parsed.geometries ?? [];
  let rows = [];
  let used = '';
  if (columns.includes('ring')) {
    const rings = geoms.filter((g) => g.kind === 'polygon');
    const source = rings.length ? rings : geoms.filter((g) => g.kind === 'line').slice(0, 1);
    if (!source.length) return { ok: false, message: 'That file has no area or line to use as a boundary.' };
    source.forEach((g, ring) => {
      const coords = g.coordinates;
      const open = coords.length > 1 && coords[0][0] === coords.at(-1)[0] && coords[0][1] === coords.at(-1)[1] ? coords.slice(0, -1) : coords;
      for (const [lon, lat] of open) rows.push(ring === 0 ? { lat: num(lat), lon: num(lon) } : { lat: num(lat), lon: num(lon), ring });
    });
    used = rings.length ? `${rings.length === 1 ? 'the boundary' : `${rings.length} rings`}` : 'the first line, as a boundary';
  } else {
    const line = geoms.find((g) => g.kind === 'line' || g.kind === 'polygon');
    if (line) {
      rows = line.coordinates.map(([lon, lat]) => ({ lat: num(lat), lon: num(lon) }));
      used = line.name ? `“${line.name}”` : 'the first line';
    } else {
      const points = geoms.filter((g) => g.kind === 'point');
      if (!points.length) return { ok: false, message: 'That file has no points to use.' };
      rows = points.map((g) => ({ ...(columns.includes('name') && g.name ? { name: g.name } : {}), lat: num(g.coordinates[0][1]), lon: num(g.coordinates[0][0]) }));
      used = `${points.length} points`;
    }
  }
  if (rows.length > max) {
    // A quiet cut would compute on part of what was brought, so it is refused.
    return { ok: false, message: `That file has ${rows.length.toLocaleString('en-US')} points; this input takes at most ${max.toLocaleString('en-US')}.` };
  }
  // A polygon input takes open rings, so closing one is not news to the reader.
  return { ok: true, rows, used, openRings: columns.includes('ring') };
}

/** A list input's rows as the form writes them: one per line, in its column order. */
export function rowsText(schema, rows) {
  const columns = Object.keys(schema?.items?.properties ?? {});
  return rows.map((row) => columns.map((c) => row[c] ?? '').join(', ').replace(/(, )+$/, '')).join('\n');
}

/** What a reader is told after an import: what was read, what was ignored, what was fixed. */
export function importReport(fileName, format, parsed, filled) {
  const parts = [];
  if (filled.ok) parts.push(`Read ${filled.rows.length.toLocaleString('en-US')} ${filled.rows.length === 1 ? 'point' : 'points'} from ${fileName} (${String(format ?? '').toUpperCase()}), using ${filled.used}.`);
  else parts.push(filled.message);
  if (parsed?.converted) parts.push(`Converted from ${parsed.converted} by the core’s inverse projection.`);
  if (parsed?.datum) parts.push(parsed.datum);
  if (parsed?.ignored?.length) parts.push(`Ignored, never fetched: ${[...new Set(parsed.ignored)].join(', ')}.`);
  const repairs = (parsed?.repairs ?? []).filter((r) => !(filled.openRings && /ring closed/.test(r)));
  if (repairs.length) parts.push(`Fixed: ${repairs.join('; ')}.`);
  return parts.join(' ');
}

/**
 * Rows from a CSV once the reader has said which columns are latitude and
 * longitude (web/io-formats, "Coordinate order and CRS safety"). Every row
 * must read as a coordinate; the first that does not is named, and a
 * latitude beyond ±90 says the columns are probably the other way round.
 */
export function csvRows(schema, parsed, latCol, lonCol) {
  const columns = Object.keys(schema?.items?.properties ?? {});
  if (!columns.includes('lat') || !columns.includes('lon')) return { ok: false, message: 'This input does not take latitude and longitude.' };
  if (latCol === lonCol || latCol < 0 || lonCol < 0) return { ok: false, message: 'Choose two different columns for latitude and longitude.' };
  const nameCol = columns.includes('name') ? (parsed.headers ?? []).findIndex((h) => /^(name|id|label|point)$/i.test(h.trim())) : -1;
  const rows = [];
  for (const [i, r] of (parsed.rows ?? []).entries()) {
    const lat = Number(r[latCol]);
    const lon = Number(r[lonCol]);
    const line = i + 2; // the header is line 1
    if (!Number.isFinite(lat) || !Number.isFinite(lon)) return { ok: false, message: `Line ${line} is not a pair of numbers: ${r[latCol] ?? ''}, ${r[lonCol] ?? ''}.` };
    if (Math.abs(lat) > 90) return { ok: false, message: `Line ${line} has latitude ${lat}, beyond ±90. The columns may be the other way round.`, swap: true };
    if (Math.abs(lon) > 180) return { ok: false, message: `Line ${line} has longitude ${lon}, beyond ±180.` };
    rows.push({ ...(nameCol >= 0 && r[nameCol] ? { name: r[nameCol] } : {}), lat: num(lat), lon: num(lon) });
  }
  if (!rows.length) return { ok: false, message: 'That file has no rows under its header.' };
  const max = schema.maxItems ?? Infinity;
  if (rows.length > max) return { ok: false, message: `That file has ${rows.length.toLocaleString('en-US')} points; this input takes at most ${max.toLocaleString('en-US')}.` };
  return { ok: true, rows, used: `columns “${parsed.headers[latCol]}” and “${parsed.headers[lonCol]}”` };
}

/**
 * Rows from a CSV whose two chosen columns are easting and northing in a
 * projected system (UTM or State Plane). Every row must be a pair of numbers;
 * the whole file is converted in one core batch, and the converted latitudes
 * and longitudes then go through the same checks as a CSV of degrees.
 */
export async function csvProjected(schema, parsed, eastCol, northCol, crs, invokeBatch) {
  if (eastCol === northCol || eastCol < 0 || northCol < 0) return { ok: false, message: 'Choose two different columns for easting and northing.' };
  const pairs = [];
  for (const [i, r] of (parsed.rows ?? []).entries()) {
    const x = Number(r[eastCol]);
    const y = Number(r[northCol]);
    if (String(r[eastCol] ?? '').trim() === '' || String(r[northCol] ?? '').trim() === '' || !Number.isFinite(x) || !Number.isFinite(y)) {
      return { ok: false, message: `Line ${i + 2} is not a pair of numbers: ${r[eastCol] ?? ''}, ${r[northCol] ?? ''}.` };
    }
    pairs.push([x, y]);
  }
  if (!pairs.length) return { ok: false, message: 'That file has no rows under its header.' };
  const done = await toGeographic(crs, pairs, invokeBatch);
  if (!done.ok) return { ok: false, message: `Line ${done.index + 2} could not be converted from ${crsLabel(crs)}: ${done.message}` };
  const n = parsed.headers.length;
  const withDegrees = {
    headers: [...parsed.headers, 'latitude', 'longitude'],
    rows: parsed.rows.map((r, i) => [...parsed.headers.map((_, c) => r[c] ?? ''), done.coords[i][1], done.coords[i][0]]),
  };
  const filled = csvRows(schema, withDegrees, n, n + 1);
  if (!filled.ok) return filled;
  return { ...filled, used: `columns “${parsed.headers[eastCol]}” and “${parsed.headers[northCol]}”`, converted: crsLabel(crs), datum: datumNote(crs) };
}
