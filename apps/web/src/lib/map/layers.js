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
 * Layers for a result: [{ kind, role, points, rings, label }]. `densify(input)`
 * runs navigation.geodesic.waypoints and returns its result, or null.
 */
export async function buildLayers(tool, args, result, densify) {
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
  if (p1.every((x) => x !== null)) layers.push({ kind: 'point', role: 'input', points: [p1], label: 'A' });
  if (two) layers.push({ kind: 'point', role: 'input', points: [p2], label: 'B' });
  // A single input point (lat, lon), like the declination or geoid tools.
  const p = [numberOf(args.lon), numberOf(args.lat)];
  if (p.every((x) => x !== null)) layers.push({ kind: 'point', role: two ? 'result' : 'input', points: [p], label: two ? 'P' : '' });
  for (const v of tool.visualization ?? []) {
    if (v.kind !== 'point') continue;
    const map = Object.fromEntries(v.map);
    const q = [at(result, map.lon), at(result, map.lat)];
    if (q.every((x) => typeof x === 'number')) layers.push({ kind: 'point', role: 'result', points: [q], label: '' });
  }
  if (kinds.has('polygon')) {
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
