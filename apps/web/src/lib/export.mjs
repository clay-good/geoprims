// Exporting a result (web/io-formats, "Supported export formats"). Pure
// functions, so every format can be checked without a browser: what a reader
// downloads is what these return.
//
// Geographic formats carry the points a result holds. A point is an output
// pair (lat, lon) or a row of a list output with those fields, which is how
// the tool contract writes coordinates everywhere else.

import { cellText } from './rows.js';

const num = (v) => (v !== null && typeof v === 'object' && 'value' in v ? v.value : v);
const isFinite_ = (v) => typeof v === 'number' && Number.isFinite(v);

const XML = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&apos;' };
export const xml = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => XML[c]);

/** A CSV cell: quoted when it holds a comma, a quote, or a line break. */
export const csvCell = (v) => {
  const s = v === null || v === undefined ? '' : String(num(v));
  return /[",\n\r]/.test(s) ? `"${s.replaceAll('"', '""')}"` : s;
};

/**
 * The latitude and longitude keys of an object, paired by what surrounds the
 * word: `lat`/`lon`, `lat2`/`lon2`, `ref_lat`/`ref_lon`. The same convention
 * the manifest uses to count a coordinate as one input.
 */
export function pairsIn(o) {
  const group = (k) => k.replace(/lat(itude)?/i, '\u0000').replace(/lo(n|ng)(gitude)?/i, '\u0000');
  const lats = Object.keys(o).filter((k) => /(^|_)lat(itude)?\d*$/i.test(k));
  const pairs = [];
  for (const lat of lats) {
    const lon = Object.keys(o).find((k) => k !== lat && /(^|_)lo(n|ng)(gitude)?\d*$/i.test(k) && group(k) === group(lat));
    if (lon) pairs.push([lat, lon]);
  }
  return pairs;
}

/**
 * The points a result holds, in order: `{name, lat, lon}`. Takes the outputs'
 * own pairs first, then any list output whose rows carry one.
 */
export function pointsOf(tool, result) {
  const out = [];
  const r = result?.result ?? {};
  // A point is named by the output that holds it ("Destination latitude"
  // names "Destination"), falling back to the tool or the row.
  const titleOf = (key) => tool.outputs.properties[key]?.title ?? '';
  const collect = (o, name, named = true) => {
    for (const [latKey, lonKey] of pairsIn(o)) {
      const lat = num(o[latKey]);
      const lon = num(o[lonKey]);
      if (!isFinite_(lat) || !isFinite_(lon)) continue;
      const fromTitle = named ? titleOf(latKey).replace(/\s*latitude\s*/i, '').trim() : '';
      out.push({ name: fromTitle || name, lat, lon });
    }
  };
  collect(r, tool.title);
  for (const [key, value] of Object.entries(r)) {
    if (!Array.isArray(value)) continue;
    value.forEach((row, i) => {
      if (row && typeof row === 'object') collect(row, row.name ?? `${key} ${i + 1}`, false);
    });
  }
  return out;
}

/** The full result, as the tool produced it. */
export const toJson = (result) => `${JSON.stringify(result, null, 2)}\n`;

/** Every scalar output as one row, or a list output as one row each. */
export function toCsv(tool, result) {
  const r = result?.result ?? {};
  const scalars = Object.entries(r).filter(([, v]) => !Array.isArray(v));
  const list = Object.entries(r).find(([, v]) => Array.isArray(v) && v.every((x) => x && typeof x === 'object'));
  if (list) {
    const [, rows] = list;
    const columns = [...new Set(rows.flatMap((row) => Object.keys(row)))];
    // A quantity column carries its unit in the header, since its cells are bare numbers.
    const unitOf = (c) => rows.find((row) => row[c] && typeof row[c] === 'object' && 'unit' in row[c])?.[c].unit;
    const header = columns.map((c) => (unitOf(c) && unitOf(c) !== '1' ? csvCell(`${c} (${unitOf(c)})`) : c));
    return [header.join(','), ...rows.map((row) => columns.map((c) => csvCell(row[c])).join(','))].join('\n') + '\n';
  }
  const names = scalars.map(([k]) => k);
  return `${names.join(',')}\n${names.map((k) => csvCell(r[k])).join(',')}\n`;
}

/** The points as GeoJSON, `[lon, lat]` per RFC 7946, with the result attached. */
export function toGeoJson(tool, result) {
  const features = pointsOf(tool, result).map((p) => ({
    type: 'Feature',
    geometry: { type: 'Point', coordinates: [p.lon, p.lat] },
    properties: { name: p.name, tool: tool.id, toolVersion: tool.version },
  }));
  return `${JSON.stringify({ type: 'FeatureCollection', features }, null, 2)}\n`;
}

/** The points as KML, which is always WGS 84 and always lon,lat,alt. */
export function toKml(tool, result) {
  const marks = pointsOf(tool, result)
    .map((p) => `    <Placemark><name>${xml(p.name)}</name><Point><coordinates>${p.lon},${p.lat},0</coordinates></Point></Placemark>`)
    .join('\n');
  return `<?xml version="1.0" encoding="UTF-8"?>
<kml xmlns="http://www.opengis.net/kml/2.2">
  <Document>
    <name>${xml(tool.title)}</name>
${marks}
  </Document>
</kml>
`;
}

/** The points as GPX waypoints, in the order the result holds them. */
export function toGpx(tool, result) {
  const points = pointsOf(tool, result);
  const wpt = points
    .map((p) => `  <wpt lat="${p.lat}" lon="${p.lon}"><name>${xml(p.name)}</name></wpt>`)
    .join('\n');
  return `<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="geoprims ${xml(tool.id)}" xmlns="http://www.topografix.com/GPX/1/1">
${wpt}
</gpx>
`;
}

/** The points as WKT: one POINT, or a MULTIPOINT when there are several. */
export function toWkt(tool, result) {
  const points = pointsOf(tool, result);
  if (!points.length) return '';
  const pair = (p) => `${p.lon} ${p.lat}`;
  return points.length === 1 ? `POINT (${pair(points[0])})\n` : `MULTIPOINT (${points.map((p) => `(${pair(p)})`).join(', ')})\n`;
}

/** The answer as a reader would read it out: the sentence, then each value. */
export function toText(tool, result, display = {}) {
  const lines = [tool.title, result.summary ?? ''].filter(Boolean);
  for (const [k, v] of Object.entries(display)) {
    lines.push(`${tool.outputs.properties[k]?.title ?? k}: ${v}`);
  }
  return `${lines.join('\n')}\n`;
}

/** A Markdown table cell: a pipe or a line break would end the cell early. */
const mdCell = (v) => String(v ?? '').replace(/\|/g, '\\|').replace(/\r?\n/g, ' ');

/** Rows of objects as a Markdown table under a heading, columns titled from the item schema. */
function rowsTable(heading, rows, items = {}, show = cellText) {
  const columns = [...new Set(rows.flatMap((r) => Object.keys(r ?? {})))];
  return [
    `### ${heading}`,
    '',
    `| ${columns.map((c) => mdCell(items[c]?.title ?? c)).join(' | ')} |`,
    `|${columns.map(() => '---').join('|')}|`,
    ...rows.map((r) => `| ${columns.map((c) => mdCell(show(r?.[c], items[c]))).join(' | ')} |`),
    '',
  ];
}

/** An input as entered, with the unit a plain number is read in (`x-unit`). */
const entered = (v, schema) => {
  const unit = schema?.['x-unit'];
  return typeof v === 'number' && unit && unit !== '1' ? `${v} ${unit}` : cellText(v, schema);
};

const isRowList = (v) => Array.isArray(v) && v.length > 0 && v.every((r) => r !== null && typeof r === 'object' && !Array.isArray(r));

/**
 * A calculation sheet for field notes or an audit file: what was entered, what
 * came out, the method, the sources with their locators, the versions, and
 * when it was made. Every input appears, a blank one with the tool's own note
 * on what it means, and list inputs and outputs (a traverse's courses and its
 * adjusted points) as tables of their own. `today` is an ISO 8601 UTC instant,
 * passed in because a result never reads a clock.
 */
export function toSheet(tool, args, result, { display = {}, today } = {}) {
  const meta = result.meta ?? {};
  const line = (k, v) => `| ${mdCell(k)} | ${mdCell(v)} |`;
  const props = tool.inputs.properties ?? {};
  const given = args ?? {};
  const inputs = [];
  const inputTables = [];
  // `options` is the call envelope (explain, units), not something entered.
  for (const k of [...new Set([...Object.keys(props), ...Object.keys(given)])].filter((k) => k !== 'options')) {
    const title = props[k]?.title ?? k;
    const v = given[k];
    if (v === undefined || v === null || v === '') inputs.push(line(title, `Not given (${props[k]?.description ?? 'optional'})`));
    else if (isRowList(v)) {
      inputs.push(line(title, `${v.length} rows, below`));
      inputTables.push(...rowsTable(title, v, props[k]?.items?.properties, entered));
    } else inputs.push(line(title, Array.isArray(v) ? v.map((x) => entered(num(x), props[k]?.items)).join(', ') : entered(num(v), props[k])));
  }
  const outputs = Object.entries(display).map(([k, v]) => line(tool.outputs.properties[k]?.title ?? k, v));
  const outputTables = Object.entries(result.result ?? {})
    .filter(([k, v]) => !(k in display) && isRowList(v))
    .flatMap(([k, v]) => rowsTable(tool.outputs.properties[k]?.title ?? k, v, tool.outputs.properties[k]?.items?.properties));
  const sources = (meta.references ?? []).map(
    (r) => `- ${[r.issuer, r.title, r.edition].filter(Boolean).join(', ')}. ${r.locator}`,
  );
  return [
    `# ${tool.title}`,
    '',
    result.summary ?? '',
    '',
    '## Inputs',
    '',
    '| Input | Value |',
    '|---|---|',
    ...inputs,
    '',
    ...inputTables,
    '## Results',
    '',
    '| Result | Value |',
    '|---|---|',
    ...outputs,
    '',
    ...outputTables,
    '## Method',
    '',
    meta.model ?? '',
    ...(meta.accuracy ? ['', `Accuracy: ${meta.accuracy}`] : []),
    ...(sources.length ? ['', '## Sources', '', ...sources] : []),
    '',
    '## Provenance',
    '',
    `- geoprims ${tool.id} ${tool.version}, core ${meta.coreVersion ?? tool.coreVersion ?? ''}`.trimEnd(),
    ...(today ? [`- Made ${today}`] : []),
    '- Planning and education aid. Not a legal survey determination and not for primary navigation.',
    '',
  ].join('\n');
}

/** Every format, with the file name and media type a download needs. */
export const FORMATS = [
  { id: 'json', label: 'JSON', extension: 'json', type: 'application/json', geographic: false },
  { id: 'geojson', label: 'GeoJSON', extension: 'geojson', type: 'application/geo+json', geographic: true },
  { id: 'kml', label: 'KML', extension: 'kml', type: 'application/vnd.google-earth.kml+xml', geographic: true },
  { id: 'gpx', label: 'GPX', extension: 'gpx', type: 'application/gpx+xml', geographic: true },
  { id: 'wkt', label: 'WKT', extension: 'wkt', type: 'text/plain', geographic: true },
  { id: 'csv', label: 'CSV', extension: 'csv', type: 'text/csv', geographic: false },
  { id: 'text', label: 'Text', extension: 'txt', type: 'text/plain', geographic: false },
  { id: 'sheet', label: 'Calculation sheet', extension: 'md', type: 'text/markdown', geographic: false },
];

/** The text of one format, or '' when a result holds nothing for it. */
export function exportText(id, { tool, args, result, display, today }) {
  switch (id) {
    case 'json':
      return toJson(result);
    case 'geojson':
      return toGeoJson(tool, result);
    case 'kml':
      return toKml(tool, result);
    case 'gpx':
      return toGpx(tool, result);
    case 'wkt':
      return toWkt(tool, result);
    case 'csv':
      return toCsv(tool, result);
    case 'text':
      return toText(tool, result, display);
    case 'sheet':
      return toSheet(tool, args, result, { display, today });
    default:
      throw new Error(`no export format ${id}`);
  }
}

/** The file name a download gets: the tool id and the format's extension. */
export const fileName = (tool, id) => `${tool.id}.${FORMATS.find((f) => f.id === id)?.extension ?? 'txt'}`;

/**
 * A workflow's one export (add-job-workflows "Exports"): its `export` names a
 * step and either that step's file output (a mission KML the export tool
 * wrote) or a format to write the step's result in (the nav log as CSV).
 * Returns { name, type, text }, or null while that step has no answer.
 */
export function workflowExport(workflow, steps, tools) {
  const e = workflow.export;
  const s = e && steps[e.step];
  if (!s || s.status !== 'ok' || !s.result?.ok) return null;
  if (e.output) {
    const r = s.result.result;
    return { name: r.filename ?? `${workflow.slug}.txt`, type: r.media_type ?? 'text/plain', text: r[e.output] };
  }
  const f = FORMATS.find((x) => x.id === e.format);
  return { name: `${workflow.slug}.${f.extension}`, type: f.type, text: exportText(e.format, { tool: tools[s.tool], args: s.input, result: s.result }) };
}
