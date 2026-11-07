// Page notices, ranked (contracts/page-chrome, "Notice stacking and priority").
// A phone has room for two notes above the answer; everything else collapses
// into one line, so the answer stays visible. Result-specific warnings are not
// here: they belong inside the answer card.

/** The contract's ranking. Lower shows first, and survives the two-visible cut. */
export const RANKS = {
  'known-issue': 1,
  'expired-model': 2,
  regulation: 3,
  experimental: 4,
  'result-change': 5,
  limitation: 6,
  'expiring-model': 7,
};

/** How many notices show in full before the rest collapse. */
export const VISIBLE = 2;

const DAY = 86_400_000;
const daysBetween = (from, to) => (Date.parse(to) - Date.parse(from)) / DAY;

/** A model is "nearing expiry" inside this many days of the date it runs out. */
export const EXPIRING_DAYS = 365;
/** A result change stays on the page for this long (trust/proof-display). */
export const RESULT_CHANGE_DAYS = 90;
/**
 * The most characters a notice carries on the page. A changelog entry is a
 * paragraph written for the changelog; printed whole above the answer, two of
 * them filled a phone screen and pushed the answer off it, which is the one
 * thing the stacking contract exists to prevent. The notice carries the lead
 * sentence and the link carries the rest.
 */
export const MAX_NOTICE_CHARS = 150;

/** The lead sentence of `text`, cut to `max` characters on a word boundary. */
export function lead(text, max = MAX_NOTICE_CHARS) {
  const first = /^.*?[.!?](?=\s|$)/.exec(String(text).trim())?.[0] ?? String(text).trim();
  if (first.length <= max) return first;
  const cut = first.slice(0, max - 1);
  return `${cut.slice(0, cut.lastIndexOf(' ') > max / 2 ? cut.lastIndexOf(' ') : cut.length).trimEnd()}\u2026`;
}

/**
 * Every notice a tool page should carry, highest priority first.
 *
 * `issue` is the open known issue or null, `changes` the changelog entries
 * naming the tool, `sources` the ledger rows it cites, and `today` the build
 * day as YYYY-MM-DD.
 */
export function noticesFor({ tool, issue, changes = [], sources = [], today }) {
  const out = [];
  if (issue) {
    out.push({
      kind: 'known-issue',
      text: `Known issue: ${issue.summary}. Workaround: ${issue.workaround}`,
      href: `/known-issues/#${issue.id}`,
      linkText: 'Known issues',
    });
  }
  for (const row of sources) {
    const end = row.validTo;
    if (!end) continue;
    const left = daysBetween(today, end);
    if (left < 0) {
      out.push({ kind: 'expired-model', text: `${row.name} ${row.currentEdition} ran out on ${end}. Results still use it.`, href: '/sources/', linkText: 'Sources' });
    } else if (left <= EXPIRING_DAYS) {
      out.push({ kind: 'expiring-model', text: `${row.name} ${row.currentEdition} runs out on ${end}.`, href: '/sources/', linkText: 'Sources' });
    }
  }
  if (tool.stability === 'experimental') {
    out.push({ kind: 'experimental', text: 'Experimental: not yet fully verified.', href: '/methodology/', linkText: 'How results are checked' });
  }
  if (tool['x-limitation']) {
    const l = tool['x-limitation'];
    out.push({ kind: 'limitation', text: `${l.simplification} ${l.instead}`, href: '/methodology/', linkText: l.governs });
  }
  for (const c of changes) {
    if (c.kind !== 'result-change') continue;
    if (daysBetween(c.date, today) > RESULT_CHANGE_DAYS) continue;
    out.push({ kind: 'result-change', text: `A published result changed on ${c.date}: ${lead(c.summary)}`, href: '/changelog/', linkText: 'Changelog' });
  }
  return out.sort((a, b) => RANKS[a.kind] - RANKS[b.kind]);
}

/** The notices shown in full, and the ones behind "N more notes". */
export const split = (notices) => ({ shown: notices.slice(0, VISIBLE), rest: notices.slice(VISIBLE) });
