// Dimension lint (trust/correctness-program layer C) over the built catalog.
// Every numeric input and output names its quantity and unit (a value of any
// quantity carries its unit in the result instead), a dimensionless field does
// not claim a unit in its title, and a unit-suffixed name (like `day_minutes`)
// agrees with the declared unit.

// Parenthesized title labels that are counts or ratios, not units.
const DIMENSIONLESS_LABELS = new Set(['%', 'percent', 'rings', 'XYZ', 'TMS']);
// Name suffixes that promise a unit.
const SUFFIXES = { ft: ['ft'], m: ['m'], km: ['km'], nm: ['NM'], kt: ['kt'], mph: ['mph'], deg: ['deg'], rad: ['rad'], s: ['s'], min: ['min'], minutes: ['min'], hours: ['h'], percent: ['1', '%'], pct: ['1', '%'] };

const isNumeric = (p) => {
  const types = [p.type].flat();
  return types.includes('number') || (p.type === 'object' && p.properties?.value?.type === 'number');
};

function fieldProblems(where, name, p) {
  const out = [];
  if (!isNumeric(p)) return out;
  const q = p['x-quantity'];
  const unit = p['x-unit'];
  if (q === 'any') return out;
  if (!q || !unit) return [`${where}: numeric field has no x-quantity and x-unit`];
  const label = /\(([^)]+)\)\s*$/.exec(p.title ?? '')?.[1];
  if (q === 'dimensionless' && label && !DIMENSIONLESS_LABELS.has(label)) {
    out.push(`${where}: the title says ${label}, but the field is dimensionless`);
  }
  const suffix = /_([a-z]+)$/.exec(name)?.[1];
  if (suffix && SUFFIXES[suffix] && !SUFFIXES[suffix].includes(unit)) {
    out.push(`${where}: the name ends in _${suffix}, but the unit is ${unit}`);
  }
  return out;
}

/** Returns one line per problem, naming the tool and field. */
export function lintDimensions(catalog) {
  const problems = [];
  const walk = (tool, side, props, prefix) => {
    for (const [k, p] of Object.entries(props ?? {})) {
      if (k === 'options') continue;
      const where = `${tool.id} ${side} ${prefix}${k}`;
      if (p.type === 'array' && p.items?.properties) walk(tool, side, p.items.properties, `${prefix}${k}[].`);
      else problems.push(...fieldProblems(where, k, p));
    }
  };
  for (const t of catalog.tools) {
    walk(t, 'input', t.inputs.properties, '');
    walk(t, 'output', t.outputs.properties, '');
  }
  return problems;
}
