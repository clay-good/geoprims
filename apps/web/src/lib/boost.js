// Personal ranking in the palette (web/command-palette, "Ranking SHALL weight
// ... recency and pinned status"). The core ranks by the words; this nudges
// tools the reader pinned or used lately up the list, a little, and only in
// the palette. The first result never moves, so Enter goes where the words
// say, and a nudge never carries a tool past a much better match.

/** How far a pinned tool may climb, and a recently used one. */
export const PIN_LIFT = 3;
export const RECENT_LIFT = 2;

/**
 * The results reordered: tool rows after the first may rise by up to their
 * lift. Filled and detected rows, and the first row, stay where they are.
 */
export function personalize(results, { pinned = [], recent = [] } = {}) {
  const lift = (r) => (r.kind !== 'tool' ? 0 : pinned.includes(r.id) ? PIN_LIFT : recent.includes(r.id) ? RECENT_LIFT : 0);
  const out = [...results];
  const firstTool = out.findIndex((r) => r.kind === 'tool');
  if (firstTool < 0) return out;
  const floor = firstTool + 1; // the top tool keeps its place
  for (let i = floor + 1; i < out.length; i++) {
    const n = lift(out[i]);
    if (!n) continue;
    let j = i;
    // Climb past plain tools only, and no further than the lift allows.
    while (j > floor && i - j < n && out[j - 1].kind === 'tool' && lift(out[j - 1]) < n) j--;
    if (j < i) out.splice(j, 0, out.splice(i, 1)[0]);
  }
  return out;
}
