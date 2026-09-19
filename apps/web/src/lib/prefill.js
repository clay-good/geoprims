// Natural-language prefill in the palette (discovery/natural-language-prefill):
// the core search fills the top tool's inputs from the question; this turns
// that into "Open with these values", or, when values are ambiguous, one
// choice per way of placing them, so the user decides instead of the parser.
// Shared by the compute worker and the tests.
const route = (id) => '/' + id.split('.').join('/') + '/';
const MAX_CHOICES = 6;

/** "30 degC" → "30 °C", for reading. */
export const readable = (v) =>
  typeof v === 'number' ? String(v) : String(v).replace(/ degC$/, ' °C').replace(/ degF$/, ' °F').replace(/ deg$/, '°');

/** Every way to give each ambiguous value its own candidate input, up to `max`. */
export function placements(ambiguous, max = MAX_CHOICES) {
  const out = [];
  const walk = (k, taken, acc) => {
    if (out.length >= max) return;
    if (k === ambiguous.length) return out.push({ ...acc });
    for (const c of ambiguous[k].candidates) {
      if (taken.has(c)) continue;
      taken.add(c);
      walk(k + 1, taken, { ...acc, [c]: ambiguous[k].value });
      taken.delete(c);
    }
  };
  walk(0, new Set(), {});
  return out;
}

/**
 * Palette actions for a search result's `prefill` and `ambiguous`, or [].
 * `tool` is its manifest; `encode(state)` is a gp_link_encode result.
 */
export async function prefillActions(top, tool, encode) {
  if (!top || !tool || (!top.prefill && !top.ambiguous)) return [];
  const label = (field) => (tool.inputs.properties[field]?.title ?? field).toLowerCase();
  const describe = (fields) => Object.entries(fields).map(([k, v]) => `${label(k)} ${readable(v)}`).join(', ');
  const link = async (fields) => {
    const out = await encode({ state: { i: fields }, flags: [] });
    return route(top.id) + (out.ok ? `#${out.result.fragment}` : '');
  };
  const known = top.prefill ?? {};
  if (!top.ambiguous) {
    return [{ title: 'Open with these values', summary: describe(known), href: await link(known), head: `From your question: ${tool.title}.`, headDetail: '' }];
  }
  const values = top.ambiguous.map((a) => a.value);
  const fields = [...new Set(top.ambiguous.flatMap((a) => a.candidates))].map(label);
  const choices = placements(top.ambiguous);
  const actions = [];
  // A bare number takes the unit of the input it is placed in.
  const withUnit = (field, v) => {
    const unit = tool.inputs.properties[field]?.['x-unit'];
    return /^[-+]?\d*\.?\d+$/.test(v) && unit && unit !== '1' ? `${v} ${unit}` : v;
  };
  for (const [k, bare] of choices.entries()) {
    const placed = Object.fromEntries(Object.entries(bare).map(([f, v]) => [f, withUnit(f, v)]));
    const all = { ...known, ...placed };
    actions.push({
      title: `Open with ${describe(placed)}`,
      summary: describe(all),
      href: await link(all),
      head: k === 0 ? `Which is which? ${values.join(' and ')} could each be ${fields.join(' or ')}.` : null,
      headDetail: '',
    });
  }
  return actions;
}
