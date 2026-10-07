// One source for every page's <title> and meta description
// (discovery/search-pages, "Titles and descriptions from one source"). Search
// engines cut a title near 60 characters and a description near 155, so the
// caps live here rather than in each template, and no page may promise
// something the build cannot prove.

import highIntent from '../../../../data/seo/high-intent-pages.json' with { type: 'json' };

export const SITE = 'geoprims';

/** Generated endpoints listed for a page of their own (search-pages, "Page-versus-endpoint rule"). */
export const HIGH_INTENT = new Set(highIntent.pages.map((p) => p.id));
const listing = new Map(highIntent.pages.map((p) => [p.id, p]));

/**
 * A tool as its page shows it. A listed endpoint's own prose and table values
 * live in its listing rather than in the core, since only its page needs
 * them; every other tool is returned as it is.
 */
export function withPageProse(t) {
  const entry = listing.get(t.id);
  return entry ? { ...t, whenToUse: entry.whenToUse, limitations: entry.limitations, table: entry.table } : t;
}
export const TITLE_MAX = 60;
export const DESCRIPTION_MAX = 155;

/** Marketing words a page may never use: the build cannot prove any of them. */
export const SUPERLATIVES = [
  'best', 'ultimate', 'fastest', 'most accurate', 'world-class', 'unbeatable',
  'number one', '#1', 'perfect', 'flawless', 'revolutionary', 'cutting-edge',
];

const SUFFIX = ` · ${SITE}`;

/** Cuts at the last word boundary that fits, with an ellipsis. */
function clip(text, max) {
  if (text.length <= max) return text;
  const cut = text.slice(0, max - 1);
  const space = cut.lastIndexOf(' ');
  return `${(space > max * 0.6 ? cut.slice(0, space) : cut).replace(/[\s,;:.]+$/, '')}…`;
}

/**
 * A page title: the name, an optional qualifier saying what it answers, then
 * the site. Over the cap the qualifier goes first, then the site name (search
 * engines show the site beside the result anyway), and only then is the name
 * itself shortened, so the name survives whole whenever it fits at all.
 */
export function title(name, qualifier = '') {
  const withQualifier = qualifier ? `${name} — ${qualifier}${SUFFIX}` : '';
  if (withQualifier && withQualifier.length <= TITLE_MAX) return withQualifier;
  const plain = `${name}${SUFFIX}`;
  if (plain.length <= TITLE_MAX) return plain;
  if (name.length <= TITLE_MAX) return name;
  return `${clip(name, TITLE_MAX - SUFFIX.length)}${SUFFIX}`;
}

/**
 * Enforces the cap on a title a page already composed: the qualifier after an
 * em dash goes first, and only then is the name shortened. This is what every
 * page goes through, so no template can ship a title search engines would cut.
 */
export function capTitle(full) {
  const text = String(full ?? '');
  if (text.length <= TITLE_MAX) return text;
  const suffix = text.endsWith(SUFFIX) ? SUFFIX : '';
  const body = suffix ? text.slice(0, -SUFFIX.length) : text;
  const name = body.split(' — ')[0];
  const dropped = `${name}${suffix}`;
  if (dropped.length <= TITLE_MAX) return dropped;
  if (name.length <= TITLE_MAX) return name;
  return `${clip(name, TITLE_MAX - suffix.length)}${suffix}`;
}

/**
 * The text's sentences. A period ends one only when a space follows and it
 * does not close an abbreviation like "U.S." or "e.g." or a capital initial,
 * so "a U.S. National Grid reference" stays one sentence while a sentence
 * ending on a unit symbol, "is 304,800.6096 m.", still ends.
 */
function sentences(text) {
  return text.split(/(?<=(?<!\.[A-Za-z]|(?:^|[\s(])[A-Z])[.!?])\s+/);
}

/**
 * A meta description: as many whole sentences of the text as fit under the
 * cap, so a short lead sentence keeps the one after it; the first sentence
 * shortened at a word boundary when even it is too long.
 */
export function description(text) {
  const trimmed = String(text ?? '').replace(/\s+/g, ' ').trim();
  if (trimmed.length <= DESCRIPTION_MAX) return trimmed;
  const parts = sentences(trimmed);
  let kept = parts[0];
  if (kept.length > DESCRIPTION_MAX) return clip(kept, DESCRIPTION_MAX);
  for (const next of parts.slice(1)) {
    if (`${kept} ${next}`.length > DESCRIPTION_MAX) break;
    kept = `${kept} ${next}`;
  }
  return kept;
}

/**
 * A hub page's description: its lead sentence, then as many of its tools'
 * names as fit under the cap, so a short lead still tells a search engine
 * what the page holds.
 */
export function listDescription(lead, names) {
  let text = String(lead).trim();
  const picked = [];
  for (const n of names) {
    const next = `${text} Includes ${[...picked, n].join(', ')}.`;
    if (next.length > DESCRIPTION_MAX) break;
    picked.push(n);
  }
  return picked.length ? `${text} Includes ${picked.join(', ')}.` : text;
}

/**
 * Whether a tool's page is indexable (contracts/routes-and-urls route map):
 * a stable tool, or an experimental one with full content, meaning it carries
 * its own "when to use this" and "limitations". The content gates then hold
 * every indexable page to the same minimums, whatever its stability. A
 * generated endpoint is indexable only when it is on the high-intent list;
 * every other one, and a deprecated tool, points elsewhere instead.
 */
export const indexableTool = (t) => {
  const page = withPageProse(t);
  return (
    !t.deprecation &&
    (t.composedOf.length === 0 || HIGH_INTENT.has(t.id)) &&
    (t.stability === 'stable' || (t.stability === 'experimental' && Boolean(page.whenToUse) && Boolean(page.limitations)))
  );
};

/** The superlative a string uses, if any. */
export function superlative(text) {
  const lower = ` ${String(text).toLowerCase()} `;
  return SUPERLATIVES.find((w) => lower.includes(w === '#1' ? '#1' : ` ${w} `) || lower.includes(`${w},`) || lower.includes(`${w}.`));
}

/**
 * Problems across the built pages: over the caps, repeated, or promising more
 * than the build can prove. `pages` is [{ route, title, description }].
 */
export function headProblems(pages) {
  const problems = [];
  const seen = new Map();
  for (const p of pages) {
    if (!p.title) problems.push(`${p.route}: no title`);
    else if (p.title.length > TITLE_MAX) problems.push(`${p.route}: title is ${p.title.length} characters (at most ${TITLE_MAX})`);
    if (!p.description) problems.push(`${p.route}: no description`);
    else if (p.description.length > DESCRIPTION_MAX) problems.push(`${p.route}: description is ${p.description.length} characters (at most ${DESCRIPTION_MAX})`);
    for (const [what, text] of [['title', p.title], ['description', p.description]]) {
      const word = superlative(text ?? '');
      if (word) problems.push(`${p.route}: the ${what} says "${word}"`);
    }
    const first = seen.get(p.title);
    if (first) problems.push(`${p.route} and ${first} share a title`);
    else seen.set(p.title, p.route);
  }
  return problems;
}
