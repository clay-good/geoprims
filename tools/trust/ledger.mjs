// Sources-ledger checks (trust/freshness), shared by the gate test and the
// /sources page: which ledger row each citation belongs to, and the problems.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

/** The ledger rows, read from the repository at `root`. */
export const readLedger = (root) => JSON.parse(readFileSync(join(root, 'data/sources-ledger.json'), 'utf8')).sources;

/** The ledger row a citation belongs to: the longest matching term wins. */
export function rowFor(ledger, ref) {
  let best = null;
  for (const row of ledger) {
    for (const term of row.matchTerms) {
      if (ref.title.includes(term) && (!best || term.length > best.term.length)) best = { row, term };
    }
  }
  return best?.row ?? null;
}

/** Every ledger row with the tools that cite it (the inverse source map). */
export function sourceMap(ledger, catalog) {
  const map = new Map(ledger.map((r) => [r.id, { row: r, tools: new Set(), citations: new Set() }]));
  for (const t of catalog.tools) {
    for (const ref of t.references) {
      const row = rowFor(ledger, ref);
      if (row) {
        map.get(row.id).tools.add(t.id);
        map.get(row.id).citations.add(`${ref.title} (${ref.edition})`);
      }
    }
  }
  return map;
}

const days = (a, b) => (Date.parse(b) - Date.parse(a)) / 86_400_000;

/**
 * Checks the catalog's citations against the ledger at `today` (YYYY-MM-DD).
 * Returns { errors, warnings } as lists of plain sentences.
 */
export function check(ledger, catalog, today) {
  const errors = [];
  const warnings = [];
  for (const t of catalog.tools) {
    for (const ref of t.references) {
      const row = rowFor(ledger, ref);
      if (!row) {
        errors.push(`${t.id} cites "${ref.title}", which has no sources-ledger row`);
        continue;
      }
      const cited = `${ref.title} ${ref.edition}`;
      if (row.editionCheck && !cited.includes(row.currentEdition) && row.editionStatus !== 'disclosed-lag') {
        errors.push(`${t.id} cites ${row.name} "${ref.edition}", but the current edition is ${row.currentEdition}`);
      }
    }
  }
  for (const row of ledger) {
    if (row.nextExpected && row.nextExpected <= today && (!row.lastVerified || row.lastVerified < row.nextExpected)) {
      errors.push(`${row.name}: next edition was expected on ${row.nextExpected}; verify and re-stamp`);
    }
    if (row.validTo) {
      const left = days(today, row.validTo);
      if (left < 0) errors.push(`${row.name} ${row.currentEdition} expired on ${row.validTo}; register its successor`);
      else if (left < 365) warnings.push(`${row.name} ${row.currentEdition} is valid through ${row.validTo}`);
    }
    if (!row.lastVerified) warnings.push(`${row.name}: never verified at the issuer`);
    else if (days(row.lastVerified, today) > 365) warnings.push(`${row.name}: last verified ${row.lastVerified}, over a year ago`);
    for (const k of ['id', 'name', 'issuer', 'currentEdition', 'freeAccessUrl', 'matchTerms', 'editionStatus', 'verificationNote']) {
      if (row[k] === undefined || row[k] === null || row[k] === '') errors.push(`ledger row ${row.id ?? '?'} needs ${k}`);
    }
    if (!['current', 'disclosed-lag', 'acknowledged-stale'].includes(row.editionStatus)) errors.push(`${row.id}: bad editionStatus ${row.editionStatus}`);
    if (row.legalStatus !== null && !['in-force', 'proposed', 'withdrawn'].includes(row.legalStatus)) errors.push(`${row.id}: bad legalStatus`);
  }
  return { errors, warnings };
}
