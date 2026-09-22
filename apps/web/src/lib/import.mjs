// Reading a file a person brings (web/io-formats: supported import formats,
// safe parsing, geometry validation). Pure functions over text, so they can
// run in a worker and be checked without a browser.
//
// Nothing here fetches anything. A file that points at a network resource —
// a KML NetworkLink, an external entity, an icon — is ignored and the ignored
// element is named in the report, because a file a reader brings must not be
// able to make their browser go and get something else.

import { crsFromName } from './crs.mjs';

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
  if (['geojson', 'json', 'kml', 'gpx', 'csv', 'tsv', 'wkt', 'wkb'].includes(ext)) return ext === 'json' ? 'geojson' : ext;
  const head = String(text ?? '').trimStart().slice(0, 400);
  if (head.startsWith('{') || head.startsWith('[')) return 'geojson';
  if (/<kml\b/i.test(head)) return 'kml';
  if (/<gpx\b/i.test(head)) return 'gpx';
  if (/^\s*(POINT|LINESTRING|POLYGON|MULTIPOINT|MULTILINESTRING|MULTIPOLYGON)\s*[ZM]*\s*\(/i.test(head)) return 'wkt';
  // Hex WKB starts with a byte-order byte: 00 or 01.
  if (/^\s*(\\x)?0[01][0-9a-f]{8,}\s*$/i.test(head) && /^[\s0-9a-fx\\]+$/i.test(String(text).trim())) return 'wkb';
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
  // A legacy `crs` member that is not WGS 84 changes what the numbers mean:
  // a WGS 84 UTM zone is converted by the core (crs.mjs), anything else refused.
  const name = doc?.crs?.properties?.name;
  const crs = name ? crsFromName(name) : null;
  if (name && !crs) return fail(`That file declares ${name}. geoprims reads WGS 84 and the WGS 84 UTM zones (EPSG 32601 to 32760); convert it to one of those first.`);
  if (crs?.kind === 'utm') out.crs = crs;
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


// ------------------------------------------------------------ WKB and EWKB

/** Hex text as bytes, or null when it is not hex. */
export function hexBytes(text) {
  const hex = String(text ?? '').trim().replace(/^\\x/i, '').replace(/\s+/g, '');
  if (!hex || hex.length % 2 || /[^0-9a-f]/i.test(hex)) return null;
  return Uint8Array.from({ length: hex.length / 2 }, (_, i) => parseInt(hex.slice(i * 2, i * 2 + 2), 16));
}

/**
 * Hex-encoded WKB or PostGIS EWKB: points, lines, polygons, and their MULTI
 * forms, in either byte order, with an optional SRID (which must be 4326) and
 * Z or M values (which are dropped: these tools work in two dimensions).
 */
export function readWkb(text) {
  const bytes = hexBytes(text);
  if (!bytes) return fail('That is not hex-encoded WKB.');
  const view = new DataView(bytes.buffer);
  const out = empty();
  let at = 0;
  const need = (n) => {
    if (at + n > bytes.length) throw new Error('The WKB ends in the middle of a geometry.');
  };
  const geometryAt = () => {
    need(5);
    const little = view.getUint8(at) === 1;
    at += 1;
    let type = view.getUint32(at, little);
    at += 4;
    // EWKB flags, then the ISO 1000/2000/3000 offsets for Z, M, ZM.
    const hasZ = (type & 0x80000000) !== 0;
    const hasM = (type & 0x40000000) !== 0;
    const hasSrid = (type & 0x20000000) !== 0;
    type &= 0x0fffffff;
    let dims = 2 + (hasZ ? 1 : 0) + (hasM ? 1 : 0);
    if (type > 1000) {
      const iso = Math.floor(type / 1000);
      dims = iso === 3 ? 4 : 3;
      type %= 1000;
    }
    if (hasSrid) {
      need(4);
      const srid = view.getUint32(at, little);
      at += 4;
      if (srid !== 4326) throw new Error(`That geometry is in SRID ${srid}. Only WGS 84 (SRID 4326) is read.`);
    }
    const point = () => {
      need(8 * dims);
      const x = view.getFloat64(at, little);
      const y = view.getFloat64(at + 8, little);
      at += 8 * dims;
      return [x, y];
    };
    const count = () => {
      need(4);
      const n = view.getUint32(at, little);
      at += 4;
      return n;
    };
    switch (type) {
      case 1:
        out.geometries.push(geometry('point', [point()]));
        break;
      case 2:
        out.geometries.push(geometry('line', Array.from({ length: count() }, point)));
        break;
      case 3:
        for (let r = count(); r > 0; r -= 1) out.geometries.push(geometry('polygon', Array.from({ length: count() }, point)));
        break;
      case 4:
      case 5:
      case 6:
      case 7:
        for (let n = count(); n > 0; n -= 1) geometryAt();
        break;
      default:
        throw new Error(`WKB geometry type ${type} is not read.`);
    }
  };
  try {
    geometryAt();
  } catch (e) {
    return fail(e.message);
  }
  return finish(out);
}

// --------------------------------------------------------------------- KMZ

/**
 * The KML inside a KMZ (a zip): the first .kml entry, usually doc.kml,
 * inflated with the platform's own DecompressionStream. Only stored and
 * deflated entries are read, and the unzipped size is capped like any file.
 */
export async function kmzText(bytes) {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const decoder = new TextDecoder();
  let at = 0;
  while (at + 30 <= bytes.length && view.getUint32(at, true) === 0x04034b50) {
    const method = view.getUint16(at + 8, true);
    const flags = view.getUint16(at + 6, true);
    let size = view.getUint32(at + 18, true);
    const nameLength = view.getUint16(at + 26, true);
    const extraLength = view.getUint16(at + 28, true);
    const name = decoder.decode(bytes.subarray(at + 30, at + 30 + nameLength));
    const start = at + 30 + nameLength + extraLength;
    if (flags & 0x08) {
      // Sizes follow the data; find them from the central directory instead.
      size = centralSize(bytes, view, name) ?? 0;
    }
    const data = bytes.subarray(start, start + size);
    if (/\.kml$/i.test(name)) {
      if (method === 0) return decoder.decode(data);
      if (method !== 8) throw new Error(`The KML in that KMZ is compressed with method ${method}, which is not read.`);
      const stream = new Blob([data]).stream().pipeThrough(new DecompressionStream('deflate-raw'));
      const inflated = new Uint8Array(await new Response(stream).arrayBuffer());
      if (inflated.length > MAX_BYTES) throw new Error(`The KML inside that KMZ is larger than ${MAX_BYTES / 1024 / 1024} MB.`);
      return decoder.decode(inflated);
    }
    at = start + size;
  }
  throw new Error('That KMZ holds no KML file.');
}

/** An entry's compressed size from the zip's central directory. */
function centralSize(bytes, view, name) {
  const decoder = new TextDecoder();
  for (let at = bytes.length - 46; at >= 0; at -= 1) {
    if (view.getUint32(at, true) !== 0x02014b50) continue;
    const nameLength = view.getUint16(at + 28, true);
    if (decoder.decode(bytes.subarray(at + 46, at + 46 + nameLength)) === name) return view.getUint32(at + 20, true);
  }
  return null;
}

/** Reads a KMZ's bytes as KML, with what was ignored and repaired. */
export async function readKmz(bytes) {
  try {
    return readKml(await kmzText(bytes));
  } catch (e) {
    return fail(e.message);
  }
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
    case 'wkb':
      return { format, ...readWkb(text) };
    case 'kmz':
      return { format, ...fail('A KMZ is read from its bytes; open it with the Import button.') };
    default:
      return { format: null, ...fail('That file is not a format this reader knows.') };
  }
}

/** Reads a file from its bytes: a KMZ is unzipped; anything else is text. */
export async function readFileBytes(name, bytes) {
  if (/\.kmz$/i.test(name ?? '') || (bytes[0] === 0x50 && bytes[1] === 0x4b)) {
    return { format: 'kmz', ...(await readKmz(bytes)) };
  }
  return readFile(name, new TextDecoder().decode(bytes));
}
