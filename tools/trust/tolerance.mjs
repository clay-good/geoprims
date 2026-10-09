// Tolerance ceilings per domain (trust/correctness-program, "Worked examples
// traced to a source"; add-trust-and-proof design T5): a golden vector field
// may be no looser than its domain's ceiling unless data/tolerance-ceilings.json
// justifies it, matched by the start of the vector's source.

const TO_M = { m: 1, km: 1000, cm: 0.01, mm: 0.001, ft: 0.3048, ftUS: 1200 / 3937, in: 0.0254, yd: 0.9144, mi: 1609.344, NM: 1852 };
const TO_DEG = { deg: 1, arcmin: 1 / 60, arcsec: 1 / 3600, rad: 180 / Math.PI };

/** The widest difference a vector's tolerance allows from `want`. */
export const bound = (t, want) => (t?.abs ?? 0) + (t?.rel ?? 0) * Math.abs(want);

/**
 * Why a field's bound breaks its domain's ceiling, or null when it does not.
 * `unit` is the result's unit at that field, if it has one; `decimals` its
 * display precision, for aviation's "one unit of display precision".
 */
export function overCeiling(domain, want, b, unit, decimals, c) {
  if (domain === 'geodesy') {
    if (unit in TO_M && b * TO_M[unit] > c.geodesy.lengthMeters) return `${(b * TO_M[unit] * 1000).toPrecision(2)} mm`;
    if (unit in TO_DEG && b * TO_DEG[unit] > c.geodesy.angleDegrees) return `${(b * TO_DEG[unit]).toPrecision(2)}°`;
    return null;
  }
  if (domain === 'aviation') {
    const unitStep = decimals === undefined ? 1 : 10 ** -decimals;
    return b > Math.max(c.aviation.relative * Math.abs(want), unitStep) ? `${b.toPrecision(2)} ${unit ?? ''}`.trim() : null;
  }
  if (domain === 'drone') return b > c.drone.relative * Math.abs(want) && b > c.drone.zero ? `${b.toPrecision(2)} ${unit ?? ''}`.trim() : null;
  if (domain === 'survey') return b > c.survey.absolute ? `${b.toPrecision(2)} ${unit ?? ''}`.trim() : null;
  if (domain === 'indexing') {
    // Exact: an integer may not move to the next one; a computed coordinate,
    // length, or area agrees to double-precision rounding across implementations.
    if (Number.isInteger(want) && !unit) return b >= 1 ? `±${b}` : null;
    return b > c.indexing.relative * Math.abs(want) + c.indexing.absolute ? `${b.toPrecision(2)} ${unit ?? ''}`.trim() : null;
  }
  return null;
}
