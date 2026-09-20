// The related-tools gate (discovery/search-pages, "Concept explainers and
// journeys"). Every stable tool points at 3 to 6 others, each with a reason a
// reader would recognise. The structural rules are enforced now; the count is
// tracked by a ratchet, because filling the lists is editorial work and the
// shortfall must only ever shrink.

/** The reasons a manifest may give (gp_base::tool::Related). */
export const REASONS = new Set(['inverse', 'next', 'alternative', 'parent']);

/** The contract's range for a stable tool's curated list. */
export const MIN_RELATED = 3;
export const MAX_RELATED = 6;

/** Structural problems with every tool's related list. */
export function relatedProblems(catalog) {
  const ids = new Set(catalog.tools.map((t) => t.id));
  const by = new Map(catalog.tools.map((t) => [t.id, t]));
  const problems = [];
  for (const t of catalog.tools) {
    const seen = new Set();
    for (const r of t.related) {
      if (!ids.has(r.id)) problems.push(`${t.id}: related ${r.id} is not a tool`);
      if (r.id === t.id) problems.push(`${t.id}: points at itself`);
      if (seen.has(r.id)) problems.push(`${t.id}: lists ${r.id} twice`);
      seen.add(r.id);
      if (!REASONS.has(r.reason)) problems.push(`${t.id}: reason ${r.reason} for ${r.id} is not one of ${[...REASONS].join(', ')}`);
      // An inverse is a two-way relationship: the other tool must say so too.
      if (r.reason === 'inverse' && ids.has(r.id)) {
        const other = by.get(r.id);
        if (!other.related.some((x) => x.id === t.id && x.reason === 'inverse')) {
          problems.push(`${t.id}: ${r.id} is its inverse but does not say so`);
        }
      }
    }
    if (t.related.length > MAX_RELATED) problems.push(`${t.id}: ${t.related.length} related tools (at most ${MAX_RELATED})`);
  }
  return problems;
}

/** Stable, self-canonical tools whose curated list is still short. */
export const short = (catalog) =>
  catalog.tools
    .filter((t) => t.stability === 'stable' && t.composedOf.length === 0 && t.related.length < MIN_RELATED)
    .map((t) => t.id)
    .sort();
