// Which list outputs the answer card shows as a table (web/tool-app "Result
// panel"). A decoder's periods and a lookup's matches are the answer; a
// thousand H3 cells are a download. The rule is here, pure, so both the page
// and its tests use the same one.

/** A list longer than this stays a count. */
export const MAX_ROWS = 20;
/** A row wider than this would not read on a phone. */
export const MAX_COLUMNS = 12;

/** A quantity cell, which reads as "119 kt". */
export const isQuantity = (x) => x !== null && typeof x === 'object' && !Array.isArray(x) && 'value' in x && 'unit' in x;

/** A cell the table can print: a plain value, or a quantity. */
export const isCell = (x) => x === null || typeof x !== 'object' || isQuantity(x);

const isRow = (v) => v !== null && typeof v === 'object' && !Array.isArray(v) && Object.values(v).every(isCell);

/** Digits grouped in threes, written out rather than read from a locale. */
const group = (digits) => digits.replace(/\B(?=(\d{3})+(?!\d))/g, ',');

/**
 * A number as its column declares it should read (`x-display-precision`):
 * a fixed number of decimals, grouped unless the column says not to, or a
 * number of significant figures. A column with no declaration prints the
 * number as the core serialized it. Display only: the result is not rounded.
 */
export function formatNumber(n, precision) {
  if (typeof n !== 'number' || !Number.isFinite(n) || !precision) return String(n);
  if (precision.significant !== undefined) return String(Number(n.toPrecision(precision.significant)));
  const fixed = n.toFixed(precision.decimals ?? 0);
  if (precision.grouping === false) return fixed;
  const [whole, frac] = fixed.split('.');
  const sign = whole.startsWith('-') ? '-' : '';
  return `${sign}${group(whole.replace('-', ''))}${frac !== undefined ? `.${frac}` : ''}`;
}

/** What one cell prints as, rounded the way its column's schema asks. */
export function cellText(x, schema) {
  const precision = schema?.['x-display-precision'];
  if (isQuantity(x)) return `${formatNumber(x.value, precision)} ${x.unit}`;
  if (typeof x === 'number') return formatNumber(x, precision);
  return x ?? '';
}

/**
 * The tables to show for a result: one per list output short enough and narrow
 * enough to read, each with its columns in the order the rows declare them.
 */
export function rowTables(result) {
  if (!result?.ok) return [];
  return Object.entries(result.result ?? {})
    .filter(([, v]) => Array.isArray(v) && v.length > 0 && v.length <= MAX_ROWS && v.every(isRow))
    .map(([key, rows]) => ({ key, rows, columns: [...new Set(rows.flatMap((r) => Object.keys(r)))] }))
    .filter((t) => t.columns.length <= MAX_COLUMNS);
}
