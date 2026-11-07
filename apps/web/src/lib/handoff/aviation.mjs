// Aviation hand-offs (web/app-shell, "Tool chaining"), loaded only for an
// aviation tool's result.

/**
 * A decoded wind handed on to the tool that uses it (aviation/weather-decoding,
 * "Wind reference is carried explicitly"): METAR and winds-aloft directions
 * are referenced to true north, so the hand-off says so, and the next tool
 * asks for the magnetic variation before it mixes that with a magnetic runway
 * or course. A METAR goes to the runway tool; a winds-aloft level, when one
 * level was decoded, to the wind triangle. A variable wind has no direction
 * to hand off. Returns { id, title, state } or null.
 */
const WIND_NOTE = 'The wind stays true north; the runway tool asks for the magnetic variation.';

export function windHandoff(fromId, result) {
  if (!result?.ok) return null;
  const r = result.result;
  const q = (v) => (v && typeof v === 'object' && Number.isFinite(v.value) ? `${v.value} ${v.unit}` : null);
  const handoff = (id, title, direction, speed, gust) =>
    direction && speed
      ? { id, title, note: WIND_NOTE, state: { i: { wind_direction: direction, wind_speed: speed, wind_reference: 'true', ...(gust ? { gust } : {}) }, c: fromId } }
      : null;
  if (fromId === 'aviation.weather.metar-decode') {
    return handoff('aviation.wind.runway-components', 'Use this wind for runway components', q(r.wind_direction), q(r.wind_speed), q(r.wind_gust));
  }
  if (fromId === 'aviation.weather.fb-winds-decode' && r.winds?.length === 1) {
    return handoff('aviation.wind.heading-groundspeed', 'Use this wind in the wind triangle', q(r.winds[0].direction), q(r.winds[0].speed));
  }
  return null;
}

/**
 * A temperature-corrected fuel density handed to the fuel weight tool
 * (aviation/fuel-density, "Feeds the fuel weight tool"): the density replaces
 * the nominal preset, and the temperature it is for goes with it, so the next
 * sentence names it. The volume goes too when one was given. `input` is what
 * produced `result`. Returns { id, title, note, state } or null.
 */
export function fuelHandoff(fromId, result, input) {
  if (fromId !== 'aviation.loading.fuel-density-temperature' || !result?.ok || !input?.temperature) return null;
  const d = result.result.density;
  if (!d || !Number.isFinite(d.value)) return null;
  const i = { density: `${d.value} ${d.unit}`, fuel_temperature: String(input.temperature), ...(input.volume ? { volume: String(input.volume) } : {}) };
  return { id: 'aviation.loading.fuel-weight', title: 'Weigh this fuel', note: 'The fuel weight tool uses this density in place of its nominal one.', state: { i, c: fromId } };
}

/** The hand-off an aviation result offers, if any. */
export const handoff = (fromId, result, input) => windHandoff(fromId, result) ?? fuelHandoff(fromId, result, input);
