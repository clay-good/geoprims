// The manifest meta-schema (define-build-contracts: closed set of manifest
// extensions). The built catalog may carry only the extension fields the
// contract lists, with values of the right shape; anything else fails the
// build naming the tool and field.

export const TOOL_EXTENSIONS = new Set([
  'x-sentence', 'x-comparison', 'x-near-margin', 'x-clock-default', 'x-primary-example',
  'x-limitation', 'x-glossary-terms', 'x-related', 'x-diagram-inline', 'x-high-intent',
]);
export const FIELD_EXTENSIONS = new Set([
  'x-quantity', 'x-unit', 'x-angle-range', 'x-display-precision', 'x-step', 'x-private',
  'x-core', 'x-help', 'x-prefill', 'x-swappable-with', 'x-status',
]);
const COMPARISONS = new Set(['vs-input', 'vs-rule-of-thumb', 'vs-typical-range', 'none']);
const STATUS_KINDS = new Set(['threshold', 'conformance']);
/** The caps the contract puts on each line of a limitation banner. */
const LIMITATION_CAPS = { simplification: 80, instead: 240, governs: 120 };
const MAX_CORE = 5;

/** Every field schema in a section, with a path, descending into list rows. */
function* fields(section, path) {
  for (const [name, f] of Object.entries(section?.properties ?? {})) {
    yield [`${path}.${name}`, f];
    if (f.items?.properties) yield* fields(f.items, `${path}.${name}[]`);
  }
}

export function metaschemaProblems(catalog, sourceIds = new Set()) {
  const problems = [];
  for (const t of catalog.tools) {
    for (const k of Object.keys(t)) {
      if (k.startsWith('x-') && !TOOL_EXTENSIONS.has(k)) problems.push(`${t.id}: unknown extension ${k}`);
    }
    const comparison = t['x-comparison'];
    const kind = comparison?.kind;
    if (comparison && !COMPARISONS.has(kind)) problems.push(`${t.id}: x-comparison kind ${kind} is not one of ${[...COMPARISONS].join(', ')}`);
    // A comparison that is not "none" carries the line it renders, and "none" carries nothing.
    if (comparison && (kind === 'none') !== (comparison.text === undefined)) {
      problems.push(`${t.id}: x-comparison kind ${kind} ${kind === 'none' ? 'takes no text' : 'needs text'}`);
    }
    if (t['x-clock-default'] && !['allowed', 'forbidden'].includes(t['x-clock-default'])) {
      problems.push(`${t.id}: x-clock-default must be allowed or forbidden`);
    }
    if (t['x-primary-example'] && !t.examples.some((e) => e.id === t['x-primary-example'])) {
      problems.push(`${t.id}: x-primary-example ${t['x-primary-example']} is not one of its examples`);
    }
    const limitation = t['x-limitation'];
    if (limitation) {
      for (const [field, cap] of Object.entries(LIMITATION_CAPS)) {
        const text = limitation[field];
        if (!text) problems.push(`${t.id}: x-limitation needs ${field}`);
        else if ([...text].length > cap) problems.push(`${t.id}: x-limitation ${field} is ${[...text].length} characters (at most ${cap})`);
      }
      for (const k of Object.keys(limitation)) {
        if (!(k in LIMITATION_CAPS)) problems.push(`${t.id}: x-limitation has no field ${k}`);
      }
    }
    let core = 0;
    for (const [side, section] of [['inputs', t.inputs], ['outputs', t.outputs]]) {
      for (const [path, f] of fields(section, side)) {
        for (const k of Object.keys(f)) {
          if (k.startsWith('x-') && !FIELD_EXTENSIONS.has(k)) problems.push(`${t.id}: unknown extension ${k} on ${path}`);
        }
        if (f['x-core'] === true) {
          if (side === 'outputs') problems.push(`${t.id}: x-core is for inputs, not ${path}`);
          // A list's row fields count with the list itself (W&B stations are one group).
          else if (!path.includes('[]')) core++;
        }
        const st = f['x-status'];
        if (st) {
          if (side === 'inputs') problems.push(`${t.id}: x-status is for outputs, not ${path}`);
          if (!STATUS_KINDS.has(st.kind)) problems.push(`${t.id}: x-status kind ${st.kind} on ${path} is not one of ${[...STATUS_KINDS].join(', ')}`);
          // "user" means the reader entered the limit; anything else cites a ledger row.
          if (st.source !== 'user' && !sourceIds.has(st.source)) problems.push(`${t.id}: x-status on ${path} cites unknown source ${st.source}`);
          if (f.type !== 'string') problems.push(`${t.id}: x-status output ${path} must be a string`);
        }
      }
    }
    if (core > MAX_CORE) problems.push(`${t.id}: ${core} x-core inputs (at most ${MAX_CORE})`);
  }
  return problems;
}
