// The stable bar (trust/correctness-program). A tool marked stable must pass
// every layer; each problem names its layer so a failed promotion says what is
// missing. Bounds fuzzing (layer D) runs over every tool in tools/fuzz, so it
// has no per-tool check here.
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { lintDimensions } from './dimensions.mjs';

export const MIN_VECTORS = 20;
export const SECTIONS = ['Method', 'Equations', 'Symbols and units', 'Domain', 'Approximations', 'Worked example', 'Differential tests', 'Invariants'];
export const EXAMPLE_FIELDS = ['sourcePublisher', 'sourceTitle', 'sourceEdition', 'sourceLocator', 'independent', 'inputs', 'outputs', 'tolerance', 'verifiedBy', 'verifiedOn'];

/** Splits a derivation note into its `## ` sections. */
export function sections(md) {
  const out = {};
  let cur = null;
  for (const line of md.split('\n')) {
    const h = /^## (.+)$/.exec(line);
    if (h) out[(cur = h[1].trim())] = '';
    else if (cur) out[cur] += line + '\n';
  }
  return out;
}

/** Layers A, B, E, and F from docs/derivations/<id>.md. */
export function derivationProblems(root, id) {
  const file = join(root, 'docs/derivations', `${id}.md`);
  if (!existsSync(file)) return [`(A) needs a derivation note at docs/derivations/${id}.md`];
  const s = sections(readFileSync(file, 'utf8'));
  const problems = [];
  for (const name of SECTIONS) {
    if (!s[name]?.trim()) problems.push(`(A) the derivation note needs a "${name}" section`);
  }
  const example = Object.fromEntries(
    [...(s['Worked example'] ?? '').matchAll(/^- (\w+): (.+)$/gm)].map(([, k, v]) => [k, v.trim()]),
  );
  for (const k of EXAMPLE_FIELDS) {
    if (!example[k]) problems.push(`(B) the worked example needs ${k}`);
  }
  if (example.independent && example.independent !== 'yes') problems.push('(B) needs an independent worked example');
  const paths = (text) => [...(text ?? '').matchAll(/^- `([^`]+)`/gm)].map((m) => m[1]);
  const diffs = paths(s['Differential tests']);
  if (s['Differential tests'] && !diffs.length) problems.push('(F) list at least one differential test file');
  for (const p of diffs) {
    if (!existsSync(join(root, p))) problems.push(`(F) differential test ${p} does not exist`);
  }
  const invariants = [...(s.Invariants ?? '').matchAll(/^- `([^`]+)` `(\w+)`/gm)];
  if (s.Invariants && !invariants.length) problems.push('(E) list at least one invariant test as `file` `test_name`');
  for (const [, p, fn] of invariants) {
    const f = join(root, p);
    if (!existsSync(f) || !new RegExp(`fn ${fn}\\(`).test(readFileSync(f, 'utf8'))) {
      problems.push(`(E) invariant test ${fn} is not in ${p}`);
    }
  }
  return problems;
}

/**
 * Every problem keeping `tool` from stable. `host` is a nodeHost with the
 * search index loaded; both surfaces rank with the same core search.
 */
export async function promotionProblems({ root, tool, host }) {
  const problems = derivationProblems(root, tool.id);
  for (const p of lintDimensions({ tools: [tool] })) problems.push(`(C) ${p}`);
  if (tool.vectorCount < MIN_VECTORS) problems.push(`needs at least ${MIN_VECTORS} golden vectors (has ${tool.vectorCount})`);
  if (tool.warnings?.includes('EXPERIMENTAL_TOOL')) problems.push('geoprims_describe still advertises the EXPERIMENTAL_TOOL warning');

  // (G) The one example the page, its button, and geoprims_run all use runs cleanly.
  const example = tool.examples.find((e) => e.id === tool['x-primary-example']) ?? tool.examples[0];
  if (!example) problems.push('(G) needs a worked example in the manifest');
  else {
    const r = JSON.parse(await host.invoke(tool.id, JSON.stringify(example.input)));
    if (!r.ok) problems.push(`(G) the example fails: ${r.error.code}`);
    for (const k of Object.keys(example.input)) {
      if (!(k in tool.inputs.properties)) problems.push(`geoprims_describe does not advertise the example's input ${k}`);
    }
  }

  // Both surfaces: the tool's own title finds it in the top 5, experimental tools hidden.
  const search = await host.module('search');
  const out = JSON.parse(await search.callString('gp_search', JSON.stringify({ query: tool.title, limit: 5 })));
  const ids = out.ok ? out.result.results.map((x) => x.id) : [];
  if (!ids.includes(tool.id)) problems.push(`searching its title "${tool.title}" does not rank it in the top 5 (got ${ids.join(', ') || 'nothing'})`);
  return problems;
}
