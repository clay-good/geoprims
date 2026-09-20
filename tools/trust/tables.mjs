// The copyrighted-table lint (trust/citations, "No reproduction of copyrighted
// tables"). Some quantities a practitioner normally reads from a table in a
// document we may not copy: AASHTO design K values by speed, ASTM and ICAO
// tables sold by their publishers. For those, a tool takes the value as an
// input and cites the table by title, edition, and number.
//
// A table that has been copied into the source looks like a long run of
// numeric literals in the module that computes one of those quantities. This
// finds those runs. It is deliberately narrow: it looks only at the tools
// flagged below, because the core is full of legitimate constant tables (ISA
// layers, unit factors, magnetic coefficients) whose sources allow it.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

/**
 * The tools whose quantities come from a table we may not reproduce, with the
 * table that governs and the input the reader supplies instead.
 */
export const FLAGGED = [
  {
    id: 'survey.curves.vertical-curve',
    source: 'core/crates/gp-survey/src/lib.rs',
    governs: 'AASHTO Green Book Table 3-34 (crest) and 3-36 (sag), by design speed',
    input: 'k',
    cite: 'Green Book',
  },
  {
    id: 'survey.curves.circular-curve',
    source: 'core/crates/gp-survey/src/lib.rs',
    governs: 'AASHTO Green Book Table 3-7 (minimum radius by design speed and superelevation)',
    // The radius comes from the project, not from the table, so the tool needs
    // no AASHTO citation — but the table must not appear in the source.
    input: 'radius',
    cite: null,
  },
  {
    id: 'aviation.altimetry.density-altitude',
    source: 'core/crates/gp-aviation/src/lib.rs',
    governs: 'ICAO Doc 7488 Table 1 (the standard atmosphere), which is sold',
    input: null,
    cite: 'ICAO',
  },
];

/** A run of numeric literals long enough to be a table someone typed in. */
export const TABLE_RUN = 8;

/**
 * Numeric-literal runs in `source` that look like a copied table. Each match is
 * {line, count, sample}. A run inside a test or a vector file is not counted:
 * this reads the tool's own module only.
 */
export function tableRuns(source) {
  const out = [];
  const body = source.split(/\n\s*#\[cfg\(test\)\]/)[0];
  for (const m of body.matchAll(/\[[^\][]*\]/g)) {
    // A table is numbers and separators and nothing else: a bracket holding
    // fields, examples, or names is a list of things, not a table of values.
    const inner = m[0].slice(1, -1);
    if (/[A-Za-z_"']/.test(inner.replace(/[eE](?=[-+]?\d)/g, '').replace(/\b(f64|f32|i32|u32|usize)\b/g, ''))) continue;
    const numbers = inner.match(/-?\d+\.?\d*(?:[eE][-+]?\d+)?/g) ?? [];
    if (numbers.length < TABLE_RUN) continue;
    // A run of small integers is an index or a shape, not a design table.
    if (numbers.every((n) => /^-?\d+$/.test(n) && Math.abs(Number(n)) < 64)) continue;
    out.push({
      line: body.slice(0, m.index).split('\n').length,
      count: numbers.length,
      sample: m[0].replace(/\s+/g, ' ').slice(0, 80),
    });
  }
  return out;
}

/**
 * Everything wrong with the flagged tools: a table-shaped run in one of their
 * modules, a missing input where the reader should supply the value, or a
 * tool that no longer cites the table that governs it.
 */
export function tableProblems(root, catalog) {
  const problems = [];
  for (const f of FLAGGED) {
    const tool = catalog.tools.find((t) => t.id === f.id);
    if (!tool) {
      problems.push(`${f.id}: flagged but not in the catalog`);
      continue;
    }
    if (f.input && !(f.input in tool.inputs.properties)) {
      problems.push(`${f.id}: ${f.input} is not an input, so the reader cannot supply the table value`);
    }
    if (f.cite) {
      const cites = tool.references.some((r) => `${r.title} ${r.issuer} ${r.locator}`.includes(f.cite));
      if (!cites) problems.push(`${f.id}: cites no source matching "${f.cite}"`);
    }
    for (const run of tableRuns(readFileSync(join(root, f.source), 'utf8'))) {
      if (runBelongsTo(readFileSync(join(root, f.source), 'utf8'), run, f.id)) {
        problems.push(`${f.id}: ${run.count} numbers at ${f.source}:${run.line} look like a copied table — ${run.sample}`);
      }
    }
  }
  return problems;
}

/**
 * Whether a run sits inside the flagged tool's own definition rather than
 * elsewhere in a module that holds many tools.
 */
function runBelongsTo(source, run, id) {
  const lines = source.split('\n');
  const at = run.line - 1;
  // The nearest tool id above the run is the tool the run belongs to.
  for (let i = at; i >= 0; i -= 1) {
    const m = /^\s*id: "([a-z0-9.-]+)",/.exec(lines[i]);
    if (m) return m[1] === id;
  }
  return false;
}
