// Tap to define (ux/glanceable-results, "Plain vocabulary and glossary"). An
// abbreviation in a page's prose becomes a button that opens its definition,
// with the source it comes from. The popover is native HTML: no script runs,
// which is what the content policy allows and what works offline.

/** The words a definition may run to, per the requirement. */
export const MAX_WORDS = 40;

const ESCAPES = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' };
export const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ESCAPES[c]);

/** The popover's id for a glossary entry. */
export const defId = (entry) => `def-${entry.id}`;

/**
 * Wraps the first occurrence of each term in `text` with the button that opens
 * its definition. Later occurrences are left alone: a reader needs the link
 * once, and a paragraph of buttons is unreadable.
 *
 * Returns the HTML and the entries actually used, so the page emits a popover
 * for those and no others.
 */
export function markTerms(text, entries) {
  const used = [];
  let html = esc(text);
  for (const entry of entries) {
    const term = esc(entry.term);
    // A whole word, not a fragment: MSL in "MSL" but not in "MSLP".
    const at = new RegExp(`(^|[^\\w-])(${term.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})(?![\\w-])`);
    const m = at.exec(html);
    if (!m) continue;
    const button = `<button type="button" class="term" popovertarget="${defId(entry)}" aria-label="What ${term} means">${term}</button>`;
    html = html.slice(0, m.index) + m[1] + button + html.slice(m.index + m[0].length);
    used.push(entry);
  }
  return { html, used };
}

/** A definition's source line: the ledger row it cites, or nothing. */
export const sourceOf = (entry, sourceById) => sourceById.get?.(entry.source) ?? null;

/** Problems with a glossary entry a page is about to show. */
export function definitionProblems(entry, sourceById) {
  const problems = [];
  const words = entry.definition.trim().split(/\s+/).length;
  if (words > MAX_WORDS) problems.push(`${entry.term}: ${words} words (at most ${MAX_WORDS})`);
  if (!entry.expansion) problems.push(`${entry.term}: no expansion`);
  if (!sourceOf(entry, sourceById)) problems.push(`${entry.term}: cites unknown source ${entry.source}`);
  return problems;
}
