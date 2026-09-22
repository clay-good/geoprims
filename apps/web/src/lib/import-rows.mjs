// From a file a reader brought to the rows of a tool's list input
// (web/io-formats: "Drop a GPX track"). The readers in import.mjs give
// geometries as [lon, lat]; a list input takes rows in its own columns. This
// is the one place the two meet, so the order of a pair is decided here and
// nowhere else.

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
  if (parsed?.ignored?.length) parts.push(`Ignored, never fetched: ${[...new Set(parsed.ignored)].join(', ')}.`);
  const repairs = (parsed?.repairs ?? []).filter((r) => !(filled.openRings && /ring closed/.test(r)));
  if (repairs.length) parts.push(`Fixed: ${repairs.join('; ')}.`);
  return parts.join(' ');
}
