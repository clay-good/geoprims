// The status-phrase gate (ux/glanceable-results, "Status phrases without false
// assurance"). Every output a manifest marks `x-status` must read as one of
// the five allowed phrases and must never claim a result is safe, unsafe,
// legal, or approved: none of that follows from the arithmetic.

/** Words a status phrase may never use. Mirrors gp_base::status::BANNED. */
export const BANNED = ['safe', 'unsafe', 'legal', 'illegal', 'approved', 'unapproved'];

/** The phrase openings the contract allows, by status kind. */
export const OPENINGS = {
  threshold: ['Within your ', 'Near your ', 'Beyond your '],
  conformance: ['Meets ', 'Does not meet '],
};

/** The banned word a phrase uses, if any. "safety" and "legality" are fine. */
export function bannedWord(text) {
  const lower = text.toLowerCase();
  return BANNED.find((w) => new RegExp(`\\b${w}\\b`).test(lower));
}

/** Every output field a tool marks with `x-status`, as [name, x-status]. */
export function statusFields(tool) {
  return Object.entries(tool.outputs?.properties ?? {})
    .filter(([, f]) => f['x-status'])
    .map(([name, f]) => [name, f['x-status']]);
}

/**
 * Problems with the status phrases a result actually produced. `result` is the
 * envelope a tool returned for one of its examples.
 */
export function statusProblems(tool, result) {
  const problems = [];
  for (const [name, meta] of statusFields(tool)) {
    const phrase = result?.result?.[name];
    if (phrase === undefined) continue; // Optional: no limit entered, no phrase.
    const where = `${tool.id}.${name}`;
    if (typeof phrase !== 'string') {
      problems.push(`${where}: a status must be text, not ${typeof phrase}`);
      continue;
    }
    const openings = OPENINGS[meta.kind] ?? [];
    if (!openings.some((o) => phrase.startsWith(o))) {
      problems.push(`${where}: "${phrase}" does not start with one of ${openings.map((o) => `"${o.trim()}"`).join(', ')}`);
    }
    const word = bannedWord(phrase);
    if (word) problems.push(`${where}: a status phrase may not say "${word}" ("${phrase}")`);
  }
  return problems;
}
