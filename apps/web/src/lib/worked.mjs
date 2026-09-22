// Worked examples that are golden vectors (web/tool-docs, "standard docs
// template ... vector-generated worked examples; the build fails when an
// example disagrees with the tool"). Where a tool's primary example is one of
// its golden vectors, the page says so, and the build checks the tool still
// gives the vector's expected values within the vector's own tolerance.

/** A value as a vector compares it: numbers by value, "5000 ft" as "5000 ft". */
function norm(v) {
  if (typeof v === 'string') {
    const m = /^\s*([-+]?\d*\.?\d+(?:e[-+]?\d+)?)\s*(.*)$/i.exec(v);
    return m ? `${Number(m[1])} ${m[2].trim()}`.trim() : v.trim();
  }
  if (typeof v === 'number') return String(v);
  if (Array.isArray(v)) return v.map(norm);
  if (v && typeof v === 'object') return Object.fromEntries(Object.keys(v).sort().map((k) => [k, norm(v[k])]));
  return v;
}

/** The live golden vector whose input is this example's, or null (superseded vectors do not count). */
export const vectorFor = (input, vectors) => vectors.find((v) => !v.supersededBy && JSON.stringify(norm(v.input)) === JSON.stringify(norm(input))) ?? null;

const at = (obj, path) => path.split('.').reduce((o, k) => (o == null ? undefined : o[k]), obj);

/**
 * Where a result departs from a vector's expectations: [] when it agrees. The
 * rules are the core vector runner's (gp-base vectors.rs): "list.*.field"
 * passes when any element matches exactly, and a number may miss by
 * abs + rel × |expected|.
 */
export function disagreements(result, vector) {
  const out = [];
  for (const [path, want] of Object.entries(vector.expect ?? {})) {
    const wild = path.indexOf('.*.');
    if (wild >= 0) {
      const list = at(result, path.slice(0, wild));
      const rest = path.slice(wild + 3);
      if (!(Array.isArray(list) && list.some((x) => JSON.stringify(at(x, rest)) === JSON.stringify(want)))) out.push(`${path}: no element is ${JSON.stringify(want)}`);
      continue;
    }
    const got = at(result, path);
    const tol = vector.tolerance?.[path];
    if (typeof want === 'number' && typeof got === 'number') {
      const room = (tol?.abs ?? 0) + (tol?.rel ?? 0) * Math.abs(want);
      if (!(Math.abs(got - want) <= room)) out.push(`${path}: ${got}, expected ${want} ± ${room}`);
    } else if (JSON.stringify(got) !== JSON.stringify(want)) out.push(`${path}: ${JSON.stringify(got)}, expected ${JSON.stringify(want)}`);
  }
  return out;
}
