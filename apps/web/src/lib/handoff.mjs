// Hand-offs (web/app-shell, "Tool chaining"): a result that is more than one
// value handed to the tool that uses it, with what the next tool needs to
// know. One domain's hand-offs load on demand, the way one domain's diagrams
// do, so a tool page carries its own chaining code and no one else's: the
// shell budget is 90 KB and every page that shows a result loads this.

const PARTS = {
  aviation: () => import('./handoff/aviation.mjs'),
  navigation: () => import('./handoff/navigation.mjs'),
  raster: () => import('./handoff/raster.mjs'),
};

/** The hand-off a tool's result offers, if any. */
export async function handoff(fromId, result, input) {
  const part = PARTS[String(fromId).split('.')[0]];
  if (!part || !result?.ok) return null;
  return (await part()).handoff(fromId, result, input) ?? null;
}
