// Raster hand-offs (web/app-shell, "Tool chaining"), loaded only for a raster
// tool's result.

/**
 * A conditioned DEM handed to the flow tools (raster/hydrology, "a conditioned
 * grid comes first"): every flow tool's limitations say to fill or breach the
 * grid before routing water over it, and the fill tool's answer is exactly
 * what they want. The grid goes on with its cell size and its no-data value,
 * so the next tool opens ready to run. `input` is what produced `result`.
 * Returns { id, title, note, state } or null.
 */
export function hydrologyHandoff(fromId, result, input) {
  if (fromId !== 'raster.hydrology.fill' || !result?.ok) return null;
  const rows = result.result?.conditioned;
  if (!Array.isArray(rows) || rows.length < 2) return null;
  const i = {
    elevations: rows.map((r) => ({ row: r.row })),
    cell_size: String(input?.cell_size ?? ''),
    ...(input?.no_data === undefined ? {} : { no_data: input.no_data }),
  };
  if (!i.cell_size) return null;
  return {
    id: 'raster.hydrology.flow-accumulation',
    title: 'Route water over the conditioned grid',
    note: 'The flow tools take the conditioned grid, not the one you entered: every pit is out of the way.',
    state: { i, c: fromId },
  };
}

/** The hand-off a raster result offers, if any. */
export const handoff = (fromId, result, input) => hydrologyHandoff(fromId, result, input);
