// Batch mode (web/io-formats, "Batch mode over CSV"): a CSV of many cases run
// through one tool, off the page, with progress and cancel, the errors listed
// by row, and the results joined back to the original columns.
//
// Pure except for the `invokeBatch` it is handed, so the whole path — mapping,
// chunking, cancelling, joining — is checked without a browser.
import { csvCell } from './export.mjs';

/** The most rows one batch may hold, per the requirement. */
export const MAX_ROWS = 100_000;
/** Rows per call to the core: small enough to report progress and stop between. */
export const CHUNK = 1_000;

/** The inputs a column can feed: every scalar input (a list is one per row). */
export function batchInputs(tool) {
  return Object.entries(tool.inputs.properties)
    .filter(([name, s]) => name !== 'options' && s.type !== 'array')
    .map(([name, s]) => ({ name, title: s.title ?? name, unit: s['x-unit'] && s['x-unit'] !== '1' ? s['x-unit'] : '', required: (tool.inputs.required ?? []).includes(name), choices: s.enum ?? null }));
}

const squash = (s) => String(s).toLowerCase().replace(/[^a-z0-9]/g, '');

/** A first guess at which column feeds which input, from the headers. */
export function suggestMapping(tool, headers) {
  const mapping = {};
  for (const input of batchInputs(tool)) {
    const want = [squash(input.name), squash(input.title)];
    const i = headers.findIndex((h) => want.includes(squash(h)));
    mapping[input.name] = i;
  }
  return mapping;
}

/**
 * The input object for each row: a mapped cell as typed, and a bare number
 * given the unit the reader chose for that column (or the input's own).
 * Empty cells are left out, so an optional input stays at its default.
 */
export function rowArgs(tool, rows, mapping, units = {}) {
  const inputs = batchInputs(tool);
  return rows.map((row) => {
    const args = {};
    for (const input of inputs) {
      const col = mapping[input.name];
      if (col === undefined || col < 0) continue;
      const cell = String(row[col] ?? '').trim();
      if (cell === '') continue;
      const unit = units[input.name] ?? input.unit;
      args[input.name] = unit && /^[-+]?\d*\.?\d+(e[-+]?\d+)?$/i.test(cell) ? `${cell} ${unit}` : cell;
    }
    return args;
  });
}

/**
 * Runs every row, CHUNK at a time, reporting progress after each chunk and
 * stopping between chunks when `signal` is aborted. Returns the envelopes in
 * row order, and whether the run was cancelled before the end.
 */
export async function runBatch(id, argsList, { invokeBatch, onProgress = () => {}, signal } = {}) {
  if (argsList.length > MAX_ROWS) throw new Error(`A batch may hold at most ${MAX_ROWS.toLocaleString('en-US')} rows.`);
  const results = [];
  for (let start = 0; start < argsList.length; start += CHUNK) {
    if (signal?.aborted) return { results, cancelled: true };
    const part = argsList.slice(start, start + CHUNK);
    // A host may hand back the core's text or the array already parsed.
    const raw = await invokeBatch(id, JSON.stringify(part));
    const out = typeof raw === 'string' ? JSON.parse(raw) : raw;
    if (!Array.isArray(out)) throw new Error(out?.error?.message ?? 'The batch could not be run.');
    results.push(...out);
    onProgress(results.length, argsList.length);
  }
  return { results, cancelled: false };
}

/** The rows that failed, with their line in the file (the header is line 1). */
export const batchErrors = (results) =>
  results.flatMap((r, i) => (r.ok ? [] : [{ row: i + 1, line: i + 2, code: r.error?.code ?? 'ERROR', message: r.error?.message ?? 'Failed.' }]));

/**
 * The export: every original column, then each output as a number with its
 * unit in the header (so a spreadsheet can use it), then an `error` column
 * that is empty for every row that worked.
 */
export function joinedCsv(tool, headers, rows, results) {
  const outputs = Object.entries(tool.outputs.properties);
  const unitOf = (key) => results.find((r) => r.ok && r.result?.[key]?.unit)?.result[key].unit;
  const outHeaders = outputs.map(([key, s]) => (unitOf(key) ? `${s.title ?? key} (${unitOf(key)})` : (s.title ?? key)));
  const lines = [[...headers, ...outHeaders, 'error'].map(csvCell).join(',')];
  rows.forEach((row, i) => {
    const r = results[i];
    const cells = outputs.map(([key]) => {
      if (!r?.ok) return '';
      const v = r.result?.[key];
      if (v === undefined || v === null) return '';
      if (typeof v === 'object' && 'value' in v) return v.value;
      return typeof v === 'object' ? JSON.stringify(v) : v;
    });
    const error = r?.ok ? '' : `${r?.error?.code ?? 'ERROR'}: ${r?.error?.message ?? 'Failed.'}`;
    lines.push([...headers.map((_, c) => row[c] ?? ''), ...cells, error].map(csvCell).join(','));
  });
  return `${lines.join('\n')}\n`;
}
