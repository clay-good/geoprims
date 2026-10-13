// The deed parser's confirmation handoff. Parsing produces a review table;
// only these confirmed geometric fields go to the plotter.
export const EDITABLE_CALL_FIELDS = [
  'kind',
  'direction',
  'distance',
  'radius',
  'arc_length',
  'delta',
  'chord_direction',
  'chord',
  'turn',
  'tangent',
];

export const reviewRows = (rows) => (rows ?? []).map((row) => ({ ...row }));

/** Calls that contain plottable geometry, stripped of parser-only evidence. */
export function plotCalls(rows) {
  return (rows ?? [])
    .filter((row) => row.kind === 'curve'
      ? (row.chord_direction && row.chord)
        || (row.tangent === 'tangent' && row.direction && row.radius && (row.delta || row.arc_length))
      : row.kind !== 'non-metric' && row.direction && row.distance)
    .map((row) => Object.fromEntries(
      EDITABLE_CALL_FIELDS
        .filter((field) => String(row[field] ?? '').trim() !== '')
        .map((field) => [field, String(row[field]).trim()]),
    ));
}

/** Splits the original deed into plain and source-linked spans without HTML. */
export function sourceParts(text, rows) {
  const lower = String(text ?? '').toLowerCase();
  const ranges = [];
  let searchAt = 0;
  for (const [call, row] of (rows ?? []).entries()) {
    const source = String(row.source ?? '');
    const start = source ? lower.indexOf(source.toLowerCase(), searchAt) : -1;
    if (start < 0) continue;
    ranges.push({ call, start, end: start + source.length });
    searchAt = start + source.length;
  }
  const out = [];
  let at = 0;
  for (const range of ranges) {
    if (range.start < at) continue;
    if (range.start > at) out.push({ text: text.slice(at, range.start), call: null });
    out.push({ text: text.slice(range.start, range.end), call: range.call });
    at = range.end;
  }
  if (at < text.length) out.push({ text: text.slice(at), call: null });
  return out.length ? out : [{ text: String(text ?? ''), call: null }];
}

export const deedPlotState = (rows) => ({
  i: { calls: plotCalls(rows) },
  c: 'survey.land.deed-parse',
});
