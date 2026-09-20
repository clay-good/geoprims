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
