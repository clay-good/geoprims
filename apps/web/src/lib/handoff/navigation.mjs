// Navigation hand-offs (web/app-shell, "Tool chaining"), loaded only for a
// navigation tool's result.

/**
 * A predicted tide handed to the datum tool (navigation/tides, "Tide results
 * feed other tools"): the window's highest predicted height, with the datum
 * it is above and the station's datum sheet, so the datum tool opens ready to
 * convert it. `input` is what produced `result`. Returns { id, title, note,
 * state } or null; nothing without a datum sheet to convert with.
 */
export function tideHandoff(fromId, result, input) {
  if (fromId !== 'navigation.tide.predict' || !result?.ok || !input?.datum_sheet) return null;
  const h = result.result.high;
  if (!h || !Number.isFinite(h.value)) return null;
  const i = { sheet: String(input.datum_sheet), height: `${h.value} ${h.unit}`, from: result.result.datum_used, ...(input.units ? { sheet_units: input.units } : {}) };
  return { id: 'navigation.tide.datums', title: 'Convert the high tide to another datum', note: `The datum tool opens with this height above ${i.from} and the station's sheet.`, state: { i, c: fromId } };
}

/** The hand-off a navigation result offers, if any. */
export const handoff = (fromId, result, input) => tideHandoff(fromId, result, input);
