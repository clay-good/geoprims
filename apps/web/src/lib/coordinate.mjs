// The coordinate field (web/app-shell, "Schema-driven input forms"). A tool
// with a latitude and a longitude should take a coordinate however the reader
// has it — degrees and minutes off a chart, an MGRS reference from a radio, a
// Plus Code from a phone — and say what it read before anything is computed.

/** The number inside a quantity, or the number itself. */
const num = (v) => (v !== null && typeof v === 'object' && 'value' in v ? v.value : v);

/**
 * The tool's latitude and longitude inputs, or null. Only a plain pair
 * counts: a tool with two points has two gestures, not one field.
 */
export function pairOf(tool) {
  const names = Object.keys(tool.inputs.properties);
  const lat = names.filter((n) => /^(lat|latitude)$/.test(n));
  const lon = names.filter((n) => /^(lon|lng|longitude)$/.test(n));
  return lat.length === 1 && lon.length === 1 ? { lat: lat[0], lon: lon[0] } : null;
}

/** Degrees as the field takes them, to the precision a coordinate deserves. */
export const degrees = (x) => `${Number(x.toFixed(7))}`;

/**
 * Reads a coordinate in any notation the catalog can decode.
 * `candidates` detects what the text might be and `run` runs the decoder, so
 * every notation is read by the same core code the palette uses.
 *
 * Returns `{ok, lat, lon, notation, ambiguous, message}`, where `ambiguous`
 * holds the other reading when the text carried no hemisphere or label.
 */
export async function readCoordinate(text, { candidates, run }) {
  const query = String(text ?? '').trim();
  if (!query) return { ok: false, message: '' };
  for (const c of await candidates(query)) {
    const decoded = await run(c.decoder.id, c.decoder.input);
    if (!decoded?.ok) continue;
    const lat = num(decoded.result.lat);
    const lon = num(decoded.result.lon);
    if (typeof lat !== 'number' || typeof lon !== 'number') continue;
    const notation = decoded.result.notation ?? c.kind;
    // The core says when it had to assume an order; the other reading is the
    // pair swapped, and only if that reading is a real latitude.
    const assumed = (decoded.meta?.warnings ?? []).some((w) => w.code === 'AMBIGUOUS_INPUT');
    const ambiguous = assumed && Math.abs(lon) <= 90 ? { lat: lon, lon: lat } : null;
    return {
      ok: true,
      lat,
      lon,
      notation,
      ambiguous,
      message: ambiguous
        ? `Read as lat, lon: ${degrees(lat)}°, ${degrees(lon)}°`
        : `Read as ${degrees(lat)}°, ${degrees(lon)}° (${notation})`,
    };
  }
  return { ok: false, message: 'That is not a coordinate this reader knows.' };
}
