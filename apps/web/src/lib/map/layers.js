// Turns a tool's visualization descriptor, inputs, and result into canvas
// layers (web/map-canvas "Layer kinds"). Lines are densified along their true
// geometry by the core itself (navigation.geodesic.waypoints), so the drawn
// path is the geodesic or rhumb line, not a straight screen segment.

/** A number from an argument value: 40.6, "40.6", or "40.6 deg". */
export function numberOf(v) {
  if (typeof v === 'number') return Number.isFinite(v) ? v : null;
  if (typeof v !== 'string') return null;
  const n = Number.parseFloat(v.replace(/[°º]/g, ' '));
  return Number.isFinite(n) ? n : null;
}

/** The layer kinds the map canvas draws. */
export const GEO_KINDS = new Set(['line-geodesic', 'line-rhumb', 'point', 'polygon', 'bbox', 'cell-set']);

/**
 * Whether a tool's page carries the map: it declares a geographic layer or
 * takes a latitude and longitude. The canvas still hides itself when the
 * current inputs give it nothing to draw.
 */
export const mapsTool = (tool) =>
  (tool.visualization ?? []).some((v) => GEO_KINDS.has(v.kind)) || ('lat' in tool.inputs.properties && 'lon' in tool.inputs.properties);

/** A layer's field mapping: an object in the catalog ({lat: "lat"}), or [key, field] pairs. */
const mapOf = (m) => (Array.isArray(m) ? Object.fromEntries(m) : (m ?? {}));

const at = (result, field) => {
  const v = result?.result?.[field];
  return typeof v === 'number' ? v : v?.value;
};

/** The first list input whose rows carry lat and lon, grouped by ring. */
function rings(tool, args) {
  for (const [name, schema] of Object.entries(tool.inputs.properties)) {
    const cols = schema.items?.properties ?? {};
    if (schema.type !== 'array' || !cols.lat || !cols.lon || !Array.isArray(args[name])) continue;
    const out = [];
    for (const row of args[name]) {
      const lat = numberOf(row.lat);
      const lon = numberOf(row.lon);
      if (lat === null || lon === null) continue;
      const r = Number.isInteger(row.ring) ? row.ring : 0;
      (out[r] ??= []).push([lon, lat]);
    }
    return out.filter(Boolean);
  }
  return [];
}

/**
 * A flight path from an output list, in order: rows of {lat, lon}, or rows
 * that hold such a list (a corridor's lines, each with its waypoints).
 */
export function outputPath(result, field) {
  const rows = result?.result?.[field];
  if (!Array.isArray(rows)) return [];
  const pts = [];
  const take = (row) => {
    const lat = numberOf(row?.lat?.value ?? row?.lat);
    const lon = numberOf(row?.lon?.value ?? row?.lon);
    if (lat !== null && lon !== null) {
      pts.push([lon, lat]);
      return;
    }
    for (const v of Object.values(row ?? {})) if (Array.isArray(v)) v.forEach(take);
  };
  rows.forEach(take);
  return pts;
}

/**
 * A flight path split into its parts: rows that carry a `part` number (one
 * battery's sortie, say) break into one line per run of the same part, so
 * each flight can be drawn apart from its neighbors. Rows without one are a
 * single part.
 */
export function outputParts(result, field) {
  const rows = result?.result?.[field];
  if (!Array.isArray(rows) || !rows.some((r) => typeof r?.part === 'number')) {
    const pts = outputPath(result, field);
    return pts.length ? [pts] : [];
  }
  const parts = [];
  let key = null;
  for (const row of rows) {
    const lat = numberOf(row?.lat?.value ?? row?.lat);
    const lon = numberOf(row?.lon?.value ?? row?.lon);
    if (lat === null || lon === null) continue;
    if (row.part !== key || !parts.length) {
      parts.push([]);
      key = row.part;
    }
    parts[parts.length - 1].push([lon, lat]);
  }
  return parts;
}

/** The most turn points a path marks; longer paths show direction only. */
export const MAX_STOPS = 400;

/** An output list's rows as [lon, lat] rings, one per (part, ring). */
export function outputRings(result, field) {
  const rows = result?.result?.[field];
  if (!Array.isArray(rows)) return [];
  const byKey = new Map();
  for (const row of rows) {
    const lat = numberOf(row.lat?.value ?? row.lat);
    const lon = numberOf(row.lon?.value ?? row.lon);
    if (lat === null || lon === null) continue;
    const key = `${row.part ?? 0}/${row.ring ?? 0}`;
    if (!byKey.has(key)) byKey.set(key, []);
    byKey.get(key).push([lon, lat]);
  }
  return [...byKey.values()].filter((r) => r.length >= 3);
}

/** The most cells a set draws; a larger answer draws its first MAX_CELLS and says so. */
export const MAX_CELLS = 1000;

/**
 * The most cells a compacted stand-in draws. The core sends one only when it
 * fits its 10,000-cell listing limit, so this is a backstop rather than a cut:
 * the 889,954-cell fill that compacts to 6,256 costs 270 ms of boundaries and
 * 37,536 vertices, which the renderer draws well inside a frame.
 */
export const MAX_COMPACTED = 10_000;

/** The cell ids a cell-set layer names: one output field holding an id or a list of {cell}. */
export function cellIds(result, field) {
  const v = result?.result?.[field];
  if (typeof v === 'string') return [v];
  return Array.isArray(v) ? v.map((c) => (typeof c === 'string' ? c : c?.cell)).filter(Boolean) : [];
}

/**
 * Layers for a result: [{ kind, role, points, rings, label }]. `densify(input)`
 * runs navigation.geodesic.waypoints and returns its result, or null. `cells`
 * (for cell-set layers) gives { boundaries(ids, grid) → [[lon, lat], …] per id
 * (grid 'h3' or 's2'), and
 * rings(origin, k) → the ids at each grid distance 0…k }, both from the core.
 * `compare` (navigation/geodesic "Comparison overlay") adds the spherical
 * great circle to a two-point line; every such line carries its path and length.
 */
export async function buildLayers(tool, args, result, densify, cells, { compare = false } = {}) {
  const kinds = new Set((tool.visualization ?? []).map((v) => v.kind));
  const layers = [];
  const p1 = [numberOf(args.lon1), numberOf(args.lat1)];
  const p2 = [numberOf(args.lon2), numberOf(args.lat2)];
  const two = p1.every((x) => x !== null) && p2.every((x) => x !== null);
  const line = async (path, role) => {
    const r = await densify({ lat1: p1[1], lon1: p1[0], lat2: p2[1], lon2: p2[0], intervals: 128, path });
    const pts = r?.ok ? r.result.points.map((q) => [q.lon.value, q.lat.value]) : [p1, p2];
    layers.push({ kind: 'line', role, points: pts, path, length: r?.ok ? r.display?.length : undefined });
  };
  if (two && (kinds.has('line-geodesic') || kinds.has('line-rhumb'))) {
    const rhumb = kinds.has('line-rhumb');
    await line(rhumb ? 'rhumb' : 'geodesic', 'result');
    // The geodesic tools show the rhumb line as a dashed comparison, and the other way round.
    await line(rhumb ? 'geodesic' : 'rhumb', 'comparison');
    if (compare) await line('great-circle', 'comparison');
  }
  // A generated flight path (a survey grid, corridor, orbit, or facade scan):
  // the output waypoints in order, with direction arrows and turn points.
  let pathDrawn = false;
  let inParts = false;
  if (!two) {
    for (const v of tool.visualization ?? []) {
      const field = (v.kind === 'line-geodesic' || v.kind === 'line-rhumb') && mapOf(v.map).path;
      // A path in parts (a mission's sorties) draws each part as its own
      // line, alternating the accent and the neutral so neighbors stand apart.
      const parts = (field ? outputParts(result, field) : []).filter((p) => p.length >= 2);
      if (!parts.length) continue;
      const n = parts.reduce((k, p) => k + p.length, 0);
      parts.forEach((pts, i) => layers.push({ kind: 'line', role: i % 2 ? 'input' : 'result', points: pts, arrows: true, stops: n <= MAX_STOPS }));
      layers.push({ kind: 'point', role: 'result', points: [parts[0][0]], label: 'Start' });
      pathDrawn = true;
      inParts ||= parts.length > 1;
    }
    // Where the camera fires along that path, when the tool reports it: the
    // photos are the mission's product, so they are drawn as their own points.
    // Marks are the few points the answer is about (swap points, waypoints
    // out of sight, ground control), drawn full size.
    for (const v of tool.visualization ?? []) {
      const map = v.kind === 'point' ? mapOf(v.map) : {};
      const shots = map.points ? outputPath(result, map.points) : [];
      if (shots.length) layers.push({ kind: 'point', role: 'detail', points: shots });
      const marks = map.marks ? outputPath(result, map.marks) : [];
      if (marks.length) layers.push({ kind: 'point', role: 'result', points: marks });
    }
    // What the path was planned over: an area (rows with rings) or a line (a
    // corridor's centerline, a facade's wall), drawn as the input.
    if (pathDrawn && !kinds.has('polygon')) {
      for (const [name, schema] of Object.entries(tool.inputs.properties)) {
        const cols = schema.items?.properties ?? {};
        if (schema.type !== 'array' || !cols.lat || !cols.lon || !Array.isArray(args[name])) continue;
        if (cols.ring) {
          const rs = rings(tool, args).filter((r) => r.length >= 3);
          if (rs.length) layers.push({ kind: 'polygon', role: 'input', rings: rs });
        } else {
          const line = args[name].map((r) => [numberOf(r.lon), numberOf(r.lat)]).filter((q) => q.every((x) => x !== null));
          // A route's waypoints are the path itself; drawing them again as the
          // input would lay the same line over itself in two roles.
          // So are a mission's sorties, which fly every input waypoint.
          const drawn = layers.find((l) => l.kind === 'line' && l.role === 'result')?.points ?? [];
          const same = inParts || (drawn.length === line.length && line.every((q, i) => Math.abs(q[0] - drawn[i][0]) < 1e-9 && Math.abs(q[1] - drawn[i][1]) < 1e-9));
          if (line.length >= 2 && !same) layers.push({ kind: 'line', role: 'input', points: line });
        }
        break;
      }
    }
  }
  // Input points name the fields they came from, so the canvas can drag them.
  const has = (f) => f in tool.inputs.properties;
  const fieldOf = (lat, lon) => (has(lat) && has(lon) ? { lat, lon } : undefined);
  if (p1.every((x) => x !== null)) layers.push({ kind: 'point', role: 'input', points: [p1], label: 'A', field: fieldOf('lat1', 'lon1') });
  if (two) layers.push({ kind: 'point', role: 'input', points: [p2], label: 'B', field: fieldOf('lat2', 'lon2') });
  // A single input point (lat, lon), like the declination or geoid tools.
  const p = [numberOf(args.lon), numberOf(args.lat)];
  if (p.every((x) => x !== null)) layers.push({ kind: 'point', role: two ? 'result' : 'input', points: [p], label: two ? 'P' : '', field: two ? undefined : fieldOf('lat', 'lon') });
  for (const v of tool.visualization ?? []) {
    if (v.kind !== 'point') continue;
    const map = mapOf(v.map);
    const q = [at(result, map.lon), at(result, map.lat)];
    if (q.every((x) => typeof x === 'number')) layers.push({ kind: 'point', role: 'result', points: [q], label: '' });
  }
  // The sun's path through the day (ux sun path "map overlay"): a sky plot
  // laid over the site, azimuth around it and elevation inward from a horizon
  // ring, with the part above the mapping threshold marked.
  const sunRows = result?.result?.path;
  if (p.every((x) => x !== null) && Array.isArray(sunRows) && sunRows.length > 1 && sunRows.every((r) => r?.azimuth && r?.elevation)) {
    const threshold = numberOf(args.threshold) ?? 30;
    const samples = sunRows.map((r) => [r.azimuth.value ?? r.azimuth, r.elevation.value ?? r.elevation]);
    layers.push({ kind: 'sky', role: 'result', points: [p], samples, threshold });
  }
  // A cross-track offset (route-geometry "Route visualization"): the track the
  // point is measured from, and the line from the point to its foot on the
  // track. The shortest line to a track meets it square, so that line is the
  // perpendicular marker. A spherical answer is drawn on the great circle.
  const footOf = (tool.visualization ?? []).map((v) => (v.kind === 'point' ? mapOf(v.map) : {})).find((m) => /^(foot|closest)_lat$/.test(m.lat ?? ''));
  const foot = footOf ? [at(result, footOf.lon), at(result, footOf.lat)] : [];
  if (footOf && p.every((x) => x !== null) && foot.every((x) => typeof x === 'number')) {
    const path = args.method === 'spherical' ? 'great-circle' : 'geodesic';
    const leg = async (a, b, intervals = 64) => {
      const r = await densify({ lat1: a[1], lon1: a[0], lat2: b[1], lon2: b[0], intervals, path });
      return r?.ok ? r.result.points.map((q) => [q.lon.value, q.lat.value]) : [a, b];
    };
    const corners = two ? [p1, p2] : (Array.isArray(args.route) ? args.route.map((r) => [numberOf(r.lon), numberOf(r.lat)]).filter((q) => q.every((x) => x !== null)) : []);
    if (corners.length >= 2 && corners.length <= 200) {
      const track = [];
      for (let i = 0; i + 1 < corners.length; i++) track.push(...(await leg(corners[i], corners[i + 1])).slice(i ? 1 : 0));
      layers.unshift({ kind: 'line', role: 'input', points: track, track: true });
    }
    layers.push({ kind: 'line', role: 'result', points: await leg(p, foot, 16), offset: true });
  }
  // A cell (bbox): its edges follow parallels and meridians, so they are
  // sampled along latitude and longitude rather than drawn as geodesics.
  for (const v of tool.visualization ?? []) {
    if (v.kind !== 'bbox') continue;
    const map = mapOf(v.map);
    const [s, w, n, e] = [map.south, map.west, map.north, map.east].map((f) => (f ? at(result, f) : undefined));
    if (![s, w, n, e].every((x) => typeof x === 'number')) continue;
    const steps = 16;
    const ring = [];
    for (let i = 0; i <= steps; i++) ring.push([w + ((e - w) * i) / steps, s]);
    for (let i = 1; i <= steps; i++) ring.push([e, s + ((n - s) * i) / steps]);
    for (let i = 1; i <= steps; i++) ring.push([e - ((e - w) * i) / steps, n]);
    for (let i = 1; i < steps; i++) ring.push([w, n - ((n - s) * i) / steps]);
    layers.push({ kind: 'polygon', role: 'result', rings: [ring] });
    // A latitude-longitude grid reference (Maidenhead, GARS, GEOREF) names one
    // cell of a regular grid, so the grid around it (grid-references "Grid
    // overlays on the canvas") is the cell's own edges three cells out.
    if (tool.id?.startsWith('geodesy.grid-ref.')) {
      const [dw, dh] = [e - w, n - s];
      const [lo0, lo1, la0, la1] = [w - 3 * dw, e + 3 * dw, Math.max(-90, s - 3 * dh), Math.min(90, n + 3 * dh)];
      const along = (a, b) => Array.from({ length: 17 }, (_, i) => [a[0] + ((b[0] - a[0]) * i) / 16, a[1] + ((b[1] - a[1]) * i) / 16]);
      for (let k = -3; k <= 4; k++) {
        layers.unshift({ kind: 'line', role: 'input', points: along([w + k * dw, la0], [w + k * dw, la1]), grid: true });
        const lat = s + k * dh;
        if (lat >= -90 && lat <= 90) layers.unshift({ kind: 'line', role: 'input', points: along([lo0, lat], [lo1, lat]), grid: true });
      }
    }
  }
  // A set of H3 cells: each cell's outline from the core. Around an origin
  // (a k-ring or ring), each cell carries its grid distance, so the drawing
  // can fade with distance and mark the origin.
  for (const v of tool.visualization ?? []) {
    if (v.kind !== 'cell-set' || !cells) continue;
    const map = mapOf(v.map);
    // A fill too large to list comes back compacted instead: the same ground,
    // in fewer and coarser cells. Drawing that is what a large answer looks
    // like, rather than an empty map.
    const full = cellIds(result, map.cells ?? map.cell);
    const compacted = full.length ? [] : cellIds(result, map.compacted);
    // A compacted set is drawn whole. The core only sends one when it fits
    // inside its own listing limit, so it is already small, and half of a
    // covering drawn is worse than none: it reads as the answer.
    const ids = full.length ? full.slice(0, MAX_CELLS) : compacted.slice(0, MAX_COMPACTED);
    if (!ids.length) continue;
    // S2 cells come from the S2 tools, H3 cells from H3's; each grid outlines its own.
    const outlines = await cells.boundaries(ids, tool.id.startsWith('indexing.s2.') ? 's2' : 'h3');
    const k = Number.isInteger(Number(args.k)) ? Number(args.k) : null;
    const origin = typeof args.cell === 'string' && k !== null ? args.cell.trim().toLowerCase() : null;
    const distance = new Map();
    if (origin && k <= 10) (await cells.rings(origin, k)).forEach((ring, d) => ring.forEach((id) => distance.set(id, d)));
    ids.forEach((id, i) => {
      if (!outlines[i]?.length) return;
      const d = distance.get(id);
      layers.push({ kind: 'polygon', role: id === origin ? 'result' : 'input', rings: [outlines[i]], cell: id, ...(compacted.length ? { compacted: true } : {}), ...(d !== undefined ? { distance: d, weight: k ? 1 - (0.7 * d) / k : 1 } : {}) });
    });
  }
  // A polygon the tool computes (a buffer, a geofence): an output list of
  // lat/lon rows grouped by part and ring, drawn over the input it came from.
  const drawn = (tool.visualization ?? []).find((v) => v.kind === 'polygon' && mapOf(v.map).rings);
  const outRings = drawn ? outputRings(result, mapOf(drawn.map).rings) : [];
  // The grid around a grid square (grid-references "Grid overlays on the
  // canvas"): rows of {line, lat, lon}, one line each, drawn under the square.
  const gridRows = drawn && mapOf(drawn.map).grid ? result?.result?.[mapOf(drawn.map).grid] : null;
  if (Array.isArray(gridRows)) {
    const lines = new Map();
    for (const r of gridRows) {
      const [lat, lon] = [numberOf(r.lat?.value ?? r.lat), numberOf(r.lon?.value ?? r.lon)];
      if (lat === null || lon === null) continue;
      if (!lines.has(r.line)) lines.set(r.line, []);
      lines.get(r.line).push([lon, lat]);
    }
    for (const pts of lines.values()) if (pts.length >= 2) layers.push({ kind: 'line', role: 'input', points: pts, grid: true });
  }
  if (outRings.length) {
    // Under a flight path the input list is the path, already drawn, not an area.
    const input = pathDrawn ? [] : rings(tool, args).filter((r) => r.length >= 3);
    if (input.length && args.shape !== 'line') layers.push({ kind: 'polygon', role: 'input', rings: input });
    layers.push({ kind: 'polygon', role: 'result', rings: outRings });
  } else if (kinds.has('polygon')) {
    const rs = rings(tool, args);
    // Edges are geodesics: densify each through the core (up to 200 edges).
    const edges = rs.reduce((n, r) => n + r.length, 0);
    const dense = edges > 200 ? rs : await Promise.all(rs.map(async (ring) => {
      const out = [];
      for (let i = 0; i < ring.length; i++) {
        const [a, b] = [ring[i], ring[(i + 1) % ring.length]];
        const r = await densify({ lat1: a[1], lon1: a[0], lat2: b[1], lon2: b[0], intervals: 16 });
        const pts = r?.ok ? r.result.points.map((q) => [q.lon.value, q.lat.value]) : [a, b];
        out.push(...pts.slice(0, -1));
      }
      return out;
    }));
    // Under a flight path, the area is what the path covers: an input.
    if (dense.length) layers.push({ kind: 'polygon', role: pathDrawn ? 'input' : 'result', rings: dense });
  }
  return layers;
}

/** Every [lon, lat] a set of layers touches, for framing the view. */
export function extent(layers) {
  // Grid lines are context around the answer: the view frames the square, not
  // the 100 km square around it.
  return layers.filter((l) => !l.grid && !l.iso).flatMap((l) => (l.kind === 'polygon' ? l.rings.flat() : l.points));
}

/**
 * The isogonic overlay (geomagnetism "Isogonic overlay") from a
 * geodesy.magnetic.isogonic result: one line per isogonic line, the agonic
 * line marked, and the compass zone edges dashed. Context, not the answer,
 * so the view still frames the reader's point.
 */
export function isogonicLayers(result) {
  if (!result?.ok) return [];
  const group = (rows, keyOf) => {
    const out = new Map();
    for (const r of rows ?? []) {
      const [lat, lon] = [numberOf(r.lat?.value ?? r.lat), numberOf(r.lon?.value ?? r.lon)];
      if (lat === null || lon === null) continue;
      if (!out.has(r.line)) out.set(r.line, { key: keyOf(r), points: [] });
      out.get(r.line).points.push([lon, lat]);
    }
    return [...out.values()].filter((l) => l.points.length >= 2);
  };
  return [
    ...group(result.result.lines, (r) => r.level?.value ?? r.level).map((l) => ({ kind: 'line', role: l.key === 0 ? 'result' : 'input', points: l.points, iso: true, level: l.key })),
    ...group(result.result.zones, (r) => r.zone).map((l) => ({ kind: 'line', role: 'comparison', points: l.points, iso: true, zone: l.key })),
  ];
}

