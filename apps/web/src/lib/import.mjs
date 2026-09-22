// Reading a file a person brings (web/io-formats: supported import formats,
// safe parsing, geometry validation). Pure functions over text, so they can
// run in a worker and be checked without a browser.
//
// Nothing here fetches anything. A file that points at a network resource —
// a KML NetworkLink, an external entity, an icon — is ignored and the ignored
// element is named in the report, because a file a reader brings must not be
// able to make their browser go and get something else.

/** The declared limits. A larger file is refused before it is parsed. */
export const MAX_BYTES = 50 * 1024 * 1024;
export const MAX_VERTICES = 1_000_000;

/** What a parse returns: geometries, what was ignored, and what was repaired. */
const empty = () => ({ ok: true, geometries: [], ignored: [], repairs: [], properties: [] });

const fail = (message) => ({ ok: false, message, geometries: [], ignored: [], repairs: [], properties: [] });

/** Tags whose content is never shown, and never followed. */
const NEVER = ['script', 'style', 'iframe', 'object', 'embed', 'NetworkLink', 'Link', 'href'];

/** Strips markup from a description so it is shown as the words it holds. */
export function inertText(html) {
  return String(html ?? '')
    .replace(/<!\[CDATA\[([\s\S]*?)\]\]>/g, '$1')
    .replace(/<(script|style)\b[^>]*>[\s\S]*?<\/\1>/gi, '')
    .replace(/<[^>]*>/g, '')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&amp;/g, '&')
    .replace(/\s+/g, ' ')
    .trim();
}

/** Refuses a file too big to parse, before parsing it. */
export function checkSize(bytes) {
  if (bytes > MAX_BYTES) {
    return `That file is ${(bytes / 1e6).toFixed(1)} MB. The limit is ${MAX_BYTES / 1024 / 1024} MB, because everything is parsed on this device.`;
  }
  return null;
}

const countVertices = (geometries) => geometries.reduce((n, g) => n + g.coordinates.length, 0);

/** The format a file looks like, from its name and its first characters. */
export function sniff(name, text) {
  const ext = /\.([a-z0-9]+)$/i.exec(name ?? '')?.[1]?.toLowerCase();
  if (ext === 'kmz') return 'kmz';
  if (['geojson', 'json', 'kml', 'gpx', 'csv', 'tsv', 'wkt'].includes(ext)) return ext === 'json' ? 'geojson' : ext;
  const head = String(text ?? '').trimStart().slice(0, 400);
  if (head.startsWith('{') || head.startsWith('[')) return 'geojson';
  if (/<kml\b/i.test(head)) return 'kml';
  if (/<gpx\b/i.test(head)) return 'gpx';
  if (/^\s*(POINT|LINESTRING|POLYGON|MULTIPOINT|MULTILINESTRING|MULTIPOLYGON)\s*[ZM]*\s*\(/i.test(head)) return 'wkt';
  if (head.includes('\t')) return 'tsv';
  if (head.includes(',')) return 'csv';
  return null;
}

/** A geometry as this module carries it: a kind and a list of [lon, lat]. */
const geometry = (kind, coordinates, name = '', description = '') => ({ kind, coordinates, name, description });

// ---------------------------------------------------------------- GeoJSON

const geojsonCoords = (g, out) => {
  switch (g?.type) {
    case 'Point':
      out.push(geometry('point', [g.coordinates]));
      break;
    case 'MultiPoint':
      for (const c of g.coordinates) out.push(geometry('point', [c]));
      break;
    case 'LineString':
      out.push(geometry('line', g.coordinates));
      break;
    case 'MultiLineString':
      for (const line of g.coordinates) out.push(geometry('line', line));
      break;
    case 'Polygon':
      for (const ring of g.coordinates) out.push(geometry('polygon', ring));
      break;
    case 'MultiPolygon':
      for (const poly of g.coordinates) for (const ring of poly) out.push(geometry('polygon', ring));
      break;
    case 'GeometryCollection':
      for (const child of g.geometries ?? []) geojsonCoords(child, out);
      break;
    default:
      break;
  }
};

export function readGeoJson(text) {
  let doc;
  try {
    doc = JSON.parse(text);
  } catch (e) {
    return fail(`That is not readable JSON: ${e.message}`);
  }
  const out = empty();
  // A legacy `crs` member that is not WGS 84 changes what the numbers mean.
  const crs = doc?.crs?.properties?.name;
  if (crs && !/CRS84|4326/i.test(crs)) return fail(`That file declares ${crs}. Only WGS 84 (CRS84) is read.`);
  const features = doc?.type === 'FeatureCollection' ? (doc.features ?? []) : [doc];
  for (const f of features) {
    const before = out.geometries.length;
    geojsonCoords(f?.type === 'Feature' ? f.geometry : f, out.geometries);
    const name = f?.properties?.name ?? f?.properties?.Name ?? '';
    for (let i = before; i < out.geometries.length; i += 1) {
      out.geometries[i].name = inertText(name);
      out.geometries[i].description = inertText(f?.properties?.description ?? '');
    }
    if (f?.properties) out.properties.push(f.properties);
  }
  return finish(out);
}

// -------------------------------------------------------------- KML and GPX

/** Every `<tag>…</tag>` body in a document, without following anything. */
const bodies = (text, tag) => [...text.matchAll(new RegExp(`<${tag}\\b[^>]*>([\\s\\S]*?)</${tag}>`, 'gi'))].map((m) => m[1]);
const firstBody = (text, tag) => bodies(text, tag)[0] ?? '';

/** KML coordinates: `lon,lat[,alt]` separated by whitespace. */
const kmlCoords = (text) =>
  text
    .trim()
    .split(/\s+/)
    .filter(Boolean)
    .map((triple) => triple.split(',').slice(0, 2).map(Number))
    .filter(([lon, lat]) => Number.isFinite(lon) && Number.isFinite(lat));

export function readKml(text) {
  const out = empty();
  // A document type declaration can carry entities; none is ever expanded.
  if (/<!DOCTYPE/i.test(text)) out.ignored.push('DOCTYPE (entities are never expanded)');
  for (const tag of NEVER) {
    if (new RegExp(`<${tag}\\b`, 'i').test(text)) out.ignored.push(tag);
  }
  for (const placemark of bodies(text, 'Placemark')) {
    const name = inertText(firstBody(placemark, 'name'));
    const description = inertText(firstBody(placemark, 'description'));
    for (const point of bodies(placemark, 'Point')) {
      out.geometries.push(geometry('point', kmlCoords(firstBody(point, 'coordinates')), name, description));
    }
    for (const line of bodies(placemark, 'LineString')) {
      out.geometries.push(geometry('line', kmlCoords(firstBody(line, 'coordinates')), name, description));
    }
    for (const poly of bodies(placemark, 'Polygon')) {
      for (const ring of bodies(poly, 'LinearRing')) {
        out.geometries.push(geometry('polygon', kmlCoords(firstBody(ring, 'coordinates')), name, description));
      }
    }
  }
  return finish(out);
}

const gpxPoints = (text, tag) =>
  [...text.matchAll(new RegExp(`<${tag}\\b[^>]*\\blat="([^"]+)"[^>]*\\blon="([^"]+)"[^>]*(?:/>|>([\\s\\S]*?)</${tag}>)`, 'gi'))].map((m) => ({
    coordinate: [Number(m[2]), Number(m[1])],
    name: inertText(firstBody(m[3] ?? '', 'name')),
  }));

export function readGpx(text) {
  const out = empty();
  if (/<!DOCTYPE/i.test(text)) out.ignored.push('DOCTYPE (entities are never expanded)');
  for (const w of gpxPoints(text, 'wpt')) out.geometries.push(geometry('point', [w.coordinate], w.name));
  // A track's name is on the track, and its points are in its segments.
  for (const [container, tag] of [['trk', 'trkpt'], ['rte', 'rtept']]) {
    for (const body of bodies(text, container)) {
      const name = inertText(firstBody(body, 'name'));
      const segments = container === 'trk' ? bodies(body, 'trkseg') : [body];
      for (const segment of segments) {
        const points = gpxPoints(segment, tag);
        if (points.length) out.geometries.push(geometry('line', points.map((p) => p.coordinate), name));
      }
    }
  }
  return finish(out);
}

// -------------------------------------------------------------------- WKT

const wktPairs = (text) =>
  text
    .split(',')
    .map((pair) => pair.trim().split(/\s+/).slice(0, 2).map(Number))
    .filter(([x, y]) => Number.isFinite(x) && Number.isFinite(y));

export function readWkt(text) {
  const out = empty();
  const source = String(text ?? '').trim();
  const kind = /^(MULTIPOLYGON|MULTILINESTRING|MULTIPOINT|POLYGON|LINESTRING|POINT)/i.exec(source)?.[1]?.toUpperCase();
  if (!kind) return fail('That is not a geometry this reader knows. It reads POINT, LINESTRING, POLYGON, and their MULTI forms.');
  const body = source.slice(source.indexOf('(') + 1, source.lastIndexOf(')'));
  if (kind === 'POINT') out.geometries.push(geometry('point', wktPairs(body)));
  else if (kind === 'LINESTRING') out.geometries.push(geometry('line', wktPairs(body)));
  else if (kind === 'MULTIPOINT') for (const p of wktPairs(body.replaceAll('(', '').replaceAll(')', ''))) out.geometries.push(geometry('point', [p]));
  else {
    // Every parenthesised group is a ring or a line.
    for (const m of body.matchAll(/\(([^()]*)\)/g)) {
      out.geometries.push(geometry(kind.includes('POLYGON') ? 'polygon' : 'line', wktPairs(m[1])));
    }
  }
  return finish(out);
}

// ------------------------------------------------------------- CSV and TSV

/** Splits one delimited line, honouring quotes. */
export function splitRow(line, delimiter) {
  const cells = [];
  let cell = '';
  let quoted = false;
  for (let i = 0; i < line.length; i += 1) {
    const c = line[i];
    if (quoted) {
      if (c === '"' && line[i + 1] === '"') {
        cell += '"';
        i += 1;
      } else if (c === '"') quoted = false;
      else cell += c;
    } else if (c === '"') quoted = true;
    else if (c === delimiter) {
      cells.push(cell);
      cell = '';
    } else cell += c;
  }
  cells.push(cell);
  return cells.map((x) => x.trim());
}

/** The columns most likely to be latitude and longitude, by their headers. */
export const suggestColumns = (headers) => ({
  lat: headers.findIndex((h) => /^(lat|latitude|y)$/i.test(h.trim())),
  lon: headers.findIndex((h) => /^(lon|lng|long|longitude|x)$/i.test(h.trim())),
});

/**
 * Reads a delimited file into rows, with the header row, the suggested
 * latitude and longitude columns, and a warning when a column named latitude
 * holds values that cannot be one.
 */
export function readDelimited(text, delimiter = ',') {
  const lines = String(text ?? '')
    .split(/\r?\n/)
    .filter((l) => l.trim() !== '');
  if (!lines.length) return { ...fail('That file has no rows.'), headers: [], rows: [] };
  const headers = splitRow(lines[0], delimiter);
  const rows = lines.slice(1).map((l) => splitRow(l, delimiter));
  const columns = suggestColumns(headers);
  const out = { ...empty(), headers, rows, columns };
  if (columns.lat >= 0) {
    const bad = rows.filter((r) => Math.abs(Number(r[columns.lat])) > 90).length;
    if (bad) {
      // A warning for the column choice, not a repair: nothing was changed.
      out.warnings = [`${bad} of ${rows.length} rows have a latitude beyond ±90. The columns may be the other way round.`];
      out.swapped = true;
    }
  }
  return out;
}

// --------------------------------------------------------- Geometry repair

const same = (a, b) => a[0] === b[0] && a[1] === b[1];

/** Twice the signed area of a ring: positive counter-clockwise. */
export const signedArea = (ring) =>
  ring.reduce((sum, [x1, y1], i) => {
    const [x2, y2] = ring[(i + 1) % ring.length];
    return sum + (x1 * y2 - x2 * y1);
  }, 0);

/** Whether two segments cross, used to find a ring that crosses itself. */
const crosses = ([p1, p2], [p3, p4]) => {
  const d = (a, b, c) => (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
  const [d1, d2, d3, d4] = [d(p3, p4, p1), d(p3, p4, p2), d(p1, p2, p3), d(p1, p2, p4)];
  return ((d1 > 0) !== (d2 > 0)) && ((d3 > 0) !== (d4 > 0));
};

/** Whether a ring crosses itself, checked pairwise (rings here are small). */
export function selfIntersects(ring) {
  for (let i = 0; i < ring.length - 1; i += 1) {
    for (let j = i + 2; j < ring.length - 1; j += 1) {
      if (i === 0 && j === ring.length - 2) continue; // the closing pair touches
      if (crosses([ring[i], ring[i + 1]], [ring[j], ring[j + 1]])) return true;
    }
  }
  return false;
}

/**
 * Checks and repairs a geometry the way RFC 7946 asks: close the ring, drop a
 * vertex that repeats the one before it, and wind the outer ring
 * counter-clockwise. Returns the geometry and what was done to it, so nothing
 * is repaired silently.
 */
export function repair(g) {
  const repairs = [];
  let coordinates = g.coordinates.filter((c, i) => i === 0 || !same(c, g.coordinates[i - 1]));
  if (coordinates.length !== g.coordinates.length) {
    repairs.push(`${g.coordinates.length - coordinates.length} repeated vertices removed`);
  }
  if (g.kind === 'polygon') {
    if (coordinates.length > 2 && !same(coordinates[0], coordinates.at(-1))) {
      coordinates = [...coordinates, coordinates[0]];
      repairs.push('ring closed');
    }
    if (coordinates.length > 3 && signedArea(coordinates.slice(0, -1)) < 0) {
      coordinates = [...coordinates].reverse();
      repairs.push('ring wound counter-clockwise, per RFC 7946');
    }
    if (selfIntersects(coordinates)) repairs.push('the ring crosses itself and was not repaired');
  }
  return { geometry: { ...g, coordinates }, repairs, valid: !repairs.some((r) => r.includes('crosses itself')) };
}

/** Applies the limits and the repairs to a parse, and reports both. */
function finish(out) {
  const vertices = countVertices(out.geometries);
  if (vertices > MAX_VERTICES) {
    return fail(`That file holds ${vertices.toLocaleString('en-US')} vertices. The limit is ${MAX_VERTICES.toLocaleString('en-US')}.`);
  }
  const geometries = [];
  for (const g of out.geometries) {
    const fixed = repair(g);
    geometries.push(fixed.geometry);
    for (const r of fixed.repairs) out.repairs.push(`${g.name || g.kind}: ${r}`);
  }
  return { ...out, geometries, vertices };
}

/** Reads a file of any supported format, by what it looks like. */
export function readFile(name, text) {
  const format = sniff(name, text);
  switch (format) {
    case 'geojson':
      return { format, ...readGeoJson(text) };
    case 'kml':
      return { format, ...readKml(text) };
    case 'gpx':
      return { format, ...readGpx(text) };
    case 'wkt':
      return { format, ...readWkt(text) };
    case 'csv':
      return { format, ...readDelimited(text, ',') };
    case 'tsv':
      return { format, ...readDelimited(text, '\t') };
    case 'kmz':
      return { format, ...fail('KMZ is a zipped KML. Unzip it and open the KML inside.') };
    default:
      return { format: null, ...fail('That file is not a format this reader knows.') };
  }
}
