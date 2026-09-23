// The stable bar (trust/correctness-program). A tool marked stable must pass
// every layer; each problem names its layer so a failed promotion says what is
// missing. Bounds fuzzing (layer D) runs over every tool in tools/fuzz, so it
// has no per-tool check here.
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { lintDimensions } from './dimensions.mjs';

export const MIN_VECTORS = 20;
/** The floor the related-tools gate holds stable tools to. */
export const MIN_RELATED = 3;
/** The prose an indexable page carries itself, per the content gate. */
export const MIN_OWN_WORDS = 150;
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
  // A preset is one call of another tool with an input fixed -- units.speed
  // .kt-to-mph is units.speed.convert with `to` set. It runs the same code
  // over the same vectors and its correctness claim is entirely its parent's,
  // so the things a claim needs -- a derivation note, twenty vectors of its
  // own, three neighbours, a page of prose -- belong to the parent and are not
  // asked of the preset. What still is asked: its own example runs, its
  // dimensions lint, and that search can find it.
  //
  // This exemption was written against `tool.parent`, which the catalog does
  // not carry; the field is `preset`. So it never fired, and the moment
  // presets began inheriting their parent's stability, 34 of them were asked
  // for derivation notes they should never have needed.
  const preset = Boolean(tool.preset);
  const problems = preset ? [] : derivationProblems(root, tool.id);
  for (const p of lintDimensions({ tools: [tool] })) problems.push(`(C) ${p}`);
  if (!preset && tool.vectorCount < MIN_VECTORS) problems.push(`needs at least ${MIN_VECTORS} golden vectors (has ${tool.vectorCount})`);
  // What the build asks of a stable tool as well as what this module asks,
  // so that "ready" means the build will take it. These live in the manifest
  // lint and the related-tools gate, and a tool that passed here and then
  // failed them cost two rounds of finding out.
  if (!preset) {
    for (const [name, v] of [['whenToUse', tool.whenToUse], ['limitations', tool.limitations]]) {
      if (!String(v ?? '').trim()) problems.push(`(A) a stable tool needs ${name}: when to reach for it, and where its answer stops`);
    }
  }
  if (!preset && (tool.composedOf?.length ?? 0) === 0 && (tool.related?.length ?? 0) < MIN_RELATED) {
    problems.push(`(A) a stable tool needs ${MIN_RELATED} related tools (has ${tool.related?.length ?? 0})`);
  }
  // A stable tool's page is indexable, and an indexable page carries its own
  // prose rather than leaning on the template around it. Counted the same way
  // apps/web/test/content.test.mjs counts it, so the two agree.
  const own = [
    tool.summary,
    tool.whenToUse,
    tool.limitations,
    tool.accuracy,
    ...(tool.related ?? []).map((r) => r.reason),
    ...(tool.examples ?? []).map((e) => `${e.title} ${e.source}`),
  ]
    .join(' ')
    .trim()
    .split(/\s+/)
    .filter(Boolean).length;
  if (!preset && own < MIN_OWN_WORDS) {
    problems.push(`(A) its page would carry ${own} words of its own prose, under the ${MIN_OWN_WORDS} an indexable page needs`);
  }

  // Only a tool already marked stable can fail on this: an experimental one
  // carries the warning because it is experimental, and drops it as part of
  // being promoted. Counting it as a blocker beforehand made the answer to
  // "is this ready?" always no, whatever else was true of the tool.
  if (tool.stability === 'stable' && tool.warnings?.includes('EXPERIMENTAL_TOOL')) {
    problems.push('geoprims_describe still advertises the EXPERIMENTAL_TOOL warning');
  }

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

  // Both surfaces: the tool's own title finds it in the top 5. Experimental
  // tools are included in the ranking, because what is being asked is whether
  // the title finds the tool, and search hides an experimental tool by design:
  // leaving them out meant a tool could never be found by the check that
  // decides whether it is ready to stop being experimental.
  const search = await host.module('search');
  const out = JSON.parse(
    await search.callString('gp_search', JSON.stringify({ query: tool.title, limit: 5, includeExperimental: true })),
  );
  const ids = out.ok ? out.result.results.map((x) => x.id) : [];
  if (!ids.includes(tool.id)) problems.push(`searching its title "${tool.title}" does not rank it in the top 5 (got ${ids.join(', ') || 'nothing'})`);
  return problems;
}
