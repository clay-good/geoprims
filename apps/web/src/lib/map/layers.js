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

/** The cell ids a cell-set layer names: one output field holding an id or a list of {cell}. */
export function cellIds(result, field) {
  const v = result?.result?.[field];
  if (typeof v === 'string') return [v];
  return Array.isArray(v) ? v.map((c) => (typeof c === 'string' ? c : c?.cell)).filter(Boolean) : [];
}

/**
 * Layers for a result: [{ kind, role, points, rings, label }]. `densify(input)`
 * runs navigation.geodesic.waypoints and returns its result, or null. `cells`
 * (for cell-set layers) gives { boundaries(ids) → [[lon, lat], …] per id, and
 * rings(origin, k) → the ids at each grid distance 0…k }, both from the core.
 */
export async function buildLayers(tool, args, result, densify, cells) {
  const kinds = new Set((tool.visualization ?? []).map((v) => v.kind));
  const layers = [];
  const p1 = [numberOf(args.lon1), numberOf(args.lat1)];
  const p2 = [numberOf(args.lon2), numberOf(args.lat2)];
  const two = p1.every((x) => x !== null) && p2.every((x) => x !== null);
  const line = async (path, role) => {
    const r = await densify({ lat1: p1[1], lon1: p1[0], lat2: p2[1], lon2: p2[0], intervals: 128, path });
    const pts = r?.ok ? r.result.points.map((q) => [q.lon.value, q.lat.value]) : [p1, p2];
    layers.push({ kind: 'line', role, points: pts });
  };
  if (two && (kinds.has('line-geodesic') || kinds.has('line-rhumb'))) {
    const rhumb = kinds.has('line-rhumb');
    await line(rhumb ? 'rhumb' : 'geodesic', 'result');
    // The geodesic tools show the rhumb line as a dashed comparison, and the other way round.
    await line(rhumb ? 'geodesic' : 'rhumb', 'comparison');
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
  }
  // A set of H3 cells: each cell's outline from the core. Around an origin
  // (a k-ring or ring), each cell carries its grid distance, so the drawing
  // can fade with distance and mark the origin.
  for (const v of tool.visualization ?? []) {
    if (v.kind !== 'cell-set' || !cells) continue;
    const map = mapOf(v.map);
    const ids = cellIds(result, map.cells ?? map.cell).slice(0, MAX_CELLS);
    if (!ids.length) continue;
    const outlines = await cells.boundaries(ids);
    const k = Number.isInteger(Number(args.k)) ? Number(args.k) : null;
    const origin = typeof args.cell === 'string' && k !== null ? args.cell.trim().toLowerCase() : null;
    const distance = new Map();
    if (origin && k <= 10) (await cells.rings(origin, k)).forEach((ring, d) => ring.forEach((id) => distance.set(id, d)));
    ids.forEach((id, i) => {
      if (!outlines[i]?.length) return;
      const d = distance.get(id);
      layers.push({ kind: 'polygon', role: id === origin ? 'result' : 'input', rings: [outlines[i]], cell: id, ...(d !== undefined ? { distance: d, weight: k ? 1 - (0.7 * d) / k : 1 } : {}) });
    });
  }
  // A polygon the tool computes (a buffer, a geofence): an output list of
  // lat/lon rows grouped by part and ring, drawn over the input it came from.
  const drawn = (tool.visualization ?? []).find((v) => v.kind === 'polygon' && mapOf(v.map).rings);
  const outRings = drawn ? outputRings(result, mapOf(drawn.map).rings) : [];
  if (outRings.length) {
    const input = rings(tool, args).filter((r) => r.length >= 3);
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
    if (dense.length) layers.push({ kind: 'polygon', role: 'result', rings: dense });
  }
  return layers;
}

/** Every [lon, lat] a set of layers touches, for framing the view. */
export function extent(layers) {
  return layers.flatMap((l) => (l.kind === 'polygon' ? l.rings.flat() : l.points));
}
