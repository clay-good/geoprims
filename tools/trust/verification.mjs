// The per-release verification report (platform/verification "Published
// accuracy statements"): every live golden vector runs through the built Wasm
// modules, and each tool reports its vector count, sources, the largest
// observed error against the reference, and the declared tolerance there.
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const at = (o, path) => path.split('.').reduce((x, k) => x?.[k], o);

/** Runs one tool's vectors; returns its report row. */
async function toolRow(root, tool, host) {
  const file = join(root, tool.vectors);
  const vectors = existsSync(file)
    ? readFileSync(file, 'utf8').split('\n').filter(Boolean).map((l) => JSON.parse(l)).filter((v) => !v.supersededBy)
    : [];
  const sources = new Map();
  let worst = null; // the numeric check closest to its tolerance
  let numeric = 0;
  let exact = 0;
  const failures = [];
  for (const v of vectors) {
    const key = v.sourceVersion ? `${v.source} (${v.sourceVersion})` : v.source;
    sources.set(key, (sources.get(key) ?? 0) + 1);
    const got = JSON.parse(await host.invoke(tool.id, JSON.stringify(v.input)));
    for (const [path, want] of Object.entries(v.expect)) {
      const actual = at(got, path);
      if (typeof want !== 'number') {
        exact++;
        if (JSON.stringify(actual) !== JSON.stringify(want)) failures.push(`${v.id} ${path}`);
        continue;
      }
      numeric++;
      const t = v.tolerance?.[path] ?? {};
      const bound = (t.abs ?? 0) + (t.rel ?? 0) * Math.abs(want);
      const error = Math.abs(actual - want);
      if (!(error <= bound)) failures.push(`${v.id} ${path}`);
      const used = bound > 0 ? error / bound : error > 0 ? Infinity : 0;
      if (!worst || used > worst.used || (used === worst.used && error > worst.error)) {
        worst = { vector: v.id, path, error, abs: t.abs ?? 0, rel: t.rel ?? 0, used };
      }
    }
  }
  const example = tool.examples.find((e) => e.id === tool['x-primary-example']) ?? tool.examples[0];
  return {
    id: tool.id,
    title: tool.title,
    version: tool.version,
    stability: tool.stability,
    vectors: vectors.length,
    checks: { numeric, exact },
    failures,
    sources: [...sources].map(([source, count]) => ({ source, count })).sort((a, b) => b.count - a.count || a.source.localeCompare(b.source)),
    worst,
    exampleSource: example?.source ?? null,
  };
}

/** The report for the build: deterministic, so the same build gives the same report. */
export async function verificationReport({ root, catalog, host }) {
  const tools = [];
  for (const t of catalog.tools) tools.push(await toolRow(root, t, host));
  return {
    version: catalog.coreVersion,
    counts: {
      tools: tools.length,
      stable: tools.filter((t) => t.stability === 'stable').length,
      vectors: tools.reduce((n, t) => n + t.vectors, 0),
      checks: tools.reduce((n, t) => n + t.checks.numeric + t.checks.exact, 0),
      failures: tools.reduce((n, t) => n + t.failures.length, 0),
    },
    tools,
  };
}
