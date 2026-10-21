// "Send to" (web/app-shell, "Tool chaining"). An answer is often the next
// tool's input: the initial course from a geodesic goes into the wind
// triangle, a density altitude goes into a climb calculation. This works out
// which tools can take a value, and builds the one link that opens the next
// tool with that value in it and a breadcrumb back.

/** At most this many destinations are offered, so the list stays readable. */
export const MAX_TARGETS = 8;

const quantityOf = (schema) => schema?.['x-quantity'];
const isPoint = (name) => /(^|_)(lat|lon|lng|latitude|longitude|northing|easting)\d*$/.test(name);

/** The words of a field's title that carry its meaning. */
const words = (title) =>
  new Set(
    String(title ?? '')
      .toLowerCase()
      .split(/[^a-z]+/)
      .filter((w) => w.length > 3 && !['from', 'with', 'into', 'value', 'your', 'this'].includes(w)),
  );
/** Whether two titles are about the same thing: a course goes into a course. */
const sameThing = (a, b) => [...words(a)].some((w) => words(b).has(w));

/**
 * The inputs of other tools that accept the same quantity as `output`.
 * Coordinates are left out: a point is two fields and a different gesture.
 */
export function chainTargets(tools, from, outputName) {
  const output = from.outputs.properties[outputName];
  const quantity = quantityOf(output);
  if (!quantity || quantity === 'any' || isPoint(outputName)) return [];
  const out = [];
  const related = (from.related ?? []).map((r) => r.id);
  const rank = (i) => (i === -1 ? related.length : i);
  for (const t of tools) {
    if (t.id === from.id) continue;
    for (const [name, schema] of Object.entries(t.inputs.properties)) {
      if (name === 'options' || schema.enum || schema.type === 'array') continue;
      if (quantityOf(schema) !== quantity || isPoint(name)) continue;
      // The first matching input of each tool: more than one is a menu of a
      // menu, and the reader can change the field once the tool is open.
      out.push({
        id: t.id,
        title: t.title,
        field: name,
        fieldTitle: schema.title,
        unit: schema['x-unit'],
        stable: t.stability === 'stable',
        // "Initial course" belongs in "Course" before it belongs in "Angle".
        same: sameThing(output.title, schema.title),
        // A tool the manifest already calls related is the likeliest next step.
        related: related.indexOf(t.id),
      });
      break;
    }
  }
  // Related first, in the order the manifest lists them; then the tools that
  // have passed the stable bar; then by name.
  out.sort(
    (a, b) =>
      Number(b.same) - Number(a.same) ||
      rank(a.related) - rank(b.related) ||
      Number(b.stable) - Number(a.stable) ||
      a.title.localeCompare(b.title),
  );
  return out.slice(0, MAX_TARGETS).map(({ stable, related, same, ...t }) => t);
}

/** The route a tool id lives at. */
export const routeOf = (id) => '/' + id.split('.').join('/') + '/';

/**
 * The state a chained link carries: the value in the target's field, and the
 * tool it came from. `value` is the displayed text ("64.5 deg"), so the target
 * reads it exactly as a person would have typed it.
 */
export const chainState = (target, value, fromId) => ({ i: { [target.field]: value }, c: fromId });

/** The link that opens `target` with `value` in it: route plus fragment. */
export const chainHref = (target, fragment) => `${routeOf(target.id)}#${fragment}`;

/** The breadcrumb a chained page shows, or null when the link carries none. */
export function cameFrom(state, tools) {
  const id = state?.c;
  if (!id) return null;
  const tool = tools.find((t) => t.id === id);
  return tool ? { id, title: tool.title, href: routeOf(id) } : null;
}
