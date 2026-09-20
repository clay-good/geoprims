// What kind of keyboard a field wants, and whether its value can go below
// zero. Shared by the form and by the gate that scans the rendered pages, so
// the two can never disagree about which fields are numeric.

/** A field the decimal keypad is for: a number, or a quantity carrying one. */
export const isNumeric = (schema) =>
  schema['x-quantity'] !== undefined ||
  schema.type === 'number' ||
  (Array.isArray(schema.type) && schema.type.includes('number'));

/** Quantities that routinely read below zero. */
export const SIGNED_QUANTITIES = new Set(['temperature', 'temperature-difference', 'vertical-speed', 'slope']);

/** Inputs whose name says which side of zero they are on. */
export const SIGNED_NAMES = /^(lat|lon|latitude|longitude)\b|^(lat|lon)[0-9_]|(^|_)(variation|declination|offset|deviation|elevation)$/;

/**
 * Whether to offer a ± toggle. Some mobile decimal keypads have no minus key,
 * so a negative longitude or an outside air temperature is otherwise untypable.
 */
export function isSigned(name, schema) {
  if (!isNumeric(schema)) return false;
  if (typeof schema.minimum === 'number') return schema.minimum < 0;
  return SIGNED_QUANTITIES.has(schema['x-quantity']) || SIGNED_NAMES.test(name);
}

/** Flips the sign of typed text, keeping any unit that came with it. */
export function flipped(text) {
  const v = String(text ?? '').trim();
  if (v === '') return '-';
  return v.startsWith('-') ? v.slice(1).trim() : `-${v}`;
}

/** The number of decimals a written number shows, after `decimal`. */
const decimalsOf = (text, decimal = '.') => String(text).split(decimal)[1]?.match(/^\d+/)?.[0].length ?? 0;

/**
 * Moves a typed value by `delta`, keeping the unit that came with it and
 * writing the result in the reader's number format: "5000 ft" plus 100 is
 * "5100 ft", and in decimal-comma "29,92 inHg" plus 0.01 is "29,93 inHg".
 * An empty or unreadable field starts from the step itself.
 *
 * The result is rounded to the decimals the value or the step needs, so a
 * 0.01 step does not leave 29.930000000000002 in the box, and it is written
 * without thousands separators, which both formats read the same way.
 */
export function stepped(text, delta, format = 'decimal-point') {
  const [decimal, group] = format === 'decimal-comma' ? [',', '.'] : ['.', ','];
  const s = String(text ?? '').trim();
  // The number at the front, however it is grouped; the rest is its unit.
  const m = /^([+\u2212-]?[\d.,\s_]*\d)(.*)$/.exec(s);
  const [written, rest] = m ? [m[1], m[2]] : ['', s];
  const n = Number(
    written
      .replaceAll(group, '')
      .replaceAll(' ', '')
      .replaceAll('_', '')
      .replace('\u2212', '-')
      .replace(decimal, '.'),
  );
  const from = Number.isFinite(n) ? n : 0;
  const places = Math.max(decimalsOf(written, decimal), decimalsOf(String(delta)));
  const moved = (from + delta).toFixed(places).replace('.', decimal);
  return (moved + (m ? rest : s && ` ${s}`)).trim();
}

/** The four Field-mode step buttons for a field, largest decrease first. */
export const stepsOf = (schema) => {
  const step = schema['x-step'];
  return step ? [-step.large, -step.small, step.small, step.large] : [];
};

/** How a step button reads: "+10", "\u22120.01". */
export const stepLabel = (delta) => (delta < 0 ? '\u2212' : '+') + String(Math.abs(delta));
