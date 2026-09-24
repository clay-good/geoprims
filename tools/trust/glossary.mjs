// The glossary gate (define-build-contracts: glossary schema). Every
// abbreviation in a tool's title, labels, help, or sentence must resolve to a
// glossary entry or a reasoned exclusion; entries are plain language (at most
// 40 words), cite a sources-ledger row, and list the tools that use them.
//
//   node tools/trust/glossary.mjs --write   refills relatedTools from the catalog
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

// A hyphenated suffix with a letter in it stays part of the term (DE-9IM);
// a numeric one does not (GLO-30 is GLO).
const TOKEN = /\b[A-Z][A-Z0-9]{1,6}(?:-[0-9]*[A-Z][A-Z0-9]*)?\b/g;

/** Text a tool shows: title, sentence, field labels, and help up to its examples. */
function texts(tool) {
  const out = [tool.title, tool['x-sentence']];
  for (const sec of [tool.inputs, tool.outputs]) {
    for (const f of Object.values(sec?.properties ?? {})) {
      out.push(f.title);
      // "Like KDEN or A2980" and "e.g. …" introduce example values, not abbreviations.
      out.push((f['x-help'] ?? '').split(/\b(?:[Ll]ike|e\.g\.)\s/)[0]);
    }
  }
  return out.filter(Boolean);
}

/** Abbreviation-shaped tokens a tool shows. */
export function abbreviations(tool) {
  const found = new Set();
  for (const t of texts(tool)) for (const m of t.matchAll(TOKEN)) found.add(m[0]);
  return found;
}

function excluded(glossary, token, toolId) {
  return glossary.notAbbreviations.some((x) => x.token === token && (!x.tools || x.tools.includes(toolId)));
}

/** Tool ids that use each glossary term, sorted. */
export function relatedTools(glossary, catalog) {
  const map = new Map(glossary.entries.map((e) => [e.term, []]));
  for (const tool of catalog.tools) {
    for (const t of abbreviations(tool)) if (map.has(t)) map.get(t).push(tool.id);
  }
  for (const v of map.values()) v.sort();
  return map;
}

/** Every problem with the glossary against the catalog and the ledger. */
export function glossaryProblems(glossary, catalog, ledgerIds) {
  const problems = [];
  const terms = new Map();
  for (const e of glossary.entries) {
    for (const k of ['id', 'term', 'expansion', 'definition', 'source', 'relatedTools']) {
      if (e[k] === undefined || e[k] === '') problems.push(`${e.term ?? e.id}: missing ${k}`);
    }
    if (terms.has(e.term)) problems.push(`${e.term}: defined twice`);
    terms.set(e.term, e);
    const words = (e.definition ?? '').trim().split(/\s+/).length;
    if (words > 40) problems.push(`${e.term}: the definition has ${words} words (at most 40)`);
    if (!ledgerIds.has(e.source)) problems.push(`${e.term}: source "${e.source}" is not a sources-ledger row`);
  }
  const related = relatedTools(glossary, catalog);
  for (const e of glossary.entries) {
    const want = related.get(e.term);
    if (JSON.stringify(e.relatedTools) !== JSON.stringify(want)) {
      problems.push(`${e.term}: relatedTools are stale; run node tools/trust/glossary.mjs --write`);
    }
  }
  for (const tool of catalog.tools) {
    for (const t of abbreviations(tool)) {
      if (!terms.has(t) && !excluded(glossary, t, tool.id)) problems.push(`${tool.id}: "${t}" has no glossary entry`);
    }
  }
  return problems;
}

export function loadGlossary(root) {
  return JSON.parse(readFileSync(join(root, 'data/glossary.json'), 'utf8'));
}

if (process.argv[2] === '--write') {
  const root = new URL('../..', import.meta.url).pathname;
  const glossary = loadGlossary(root);
  const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
  const related = relatedTools(glossary, catalog);
  for (const e of glossary.entries) e.relatedTools = related.get(e.term);
  writeFileSync(join(root, 'data/glossary.json'), JSON.stringify(glossary, null, 2) + '\n');
}
