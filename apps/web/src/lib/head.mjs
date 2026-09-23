// One source for every page's <title> and meta description
// (discovery/search-pages, "Titles and descriptions from one source"). Search
// engines cut a title near 60 characters and a description near 155, so the
// caps live here rather than in each template, and no page may promise
// something the build cannot prove.

export const SITE = 'geoprims';
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
 * the site. Over the cap the qualifier goes first, and only then is the name
 * itself shortened, so the name always survives.
 */
export function title(name, qualifier = '') {
  const withQualifier = qualifier ? `${name} — ${qualifier}${SUFFIX}` : '';
  if (withQualifier && withQualifier.length <= TITLE_MAX) return withQualifier;
  const plain = `${name}${SUFFIX}`;
  if (plain.length <= TITLE_MAX) return plain;
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
  return `${clip(name, TITLE_MAX - suffix.length)}${suffix}`;
}

/**
 * A meta description: the first sentence of the text, shortened to the cap at
 * a word boundary when that sentence is still too long.
 */
export function description(text) {
  const trimmed = String(text ?? '').replace(/\s+/g, ' ').trim();
  if (trimmed.length <= DESCRIPTION_MAX) return trimmed;
  const stop = trimmed.search(/\.\s/);
  const first = stop > 0 ? trimmed.slice(0, stop + 1) : trimmed;
  return first.length <= DESCRIPTION_MAX ? first : clip(first, DESCRIPTION_MAX);
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
