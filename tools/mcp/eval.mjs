// The MCP evaluation suite (add-local-mcp-server, "Agent evaluation"). It
// measures what an agent actually experiences: given a practitioner's
// question, does the server surface the right tool, does running what it
// surfaced give the same answer as running the intended tool, and what does
// the round trip cost in tokens.
//
// This is not a correctness suite: whether an answer is right is settled by
// the golden vectors. This measures whether an agent can get to it.
import { generate } from '../search/accuracy.mjs';

/** Roughly four characters to the token, the usual working estimate. */
export const tokensOf = (payload) => Math.ceil(JSON.stringify(payload).length / 4);

/**
 * One task per generated question: the phrasing, the tool it means, and the
 * arguments that tool would be run with.
 */
export function tasks(catalog) {
  return generate(catalog.tools).map(({ query, id, expect }) => ({ query, id, expect }));
}

/**
 * Runs every task through the server's own handlers and reports how an agent
 * would fare. `handlers` is the object metaHandlers returns.
 */
export async function evaluate({ catalog, handlers, limit = Infinity }) {
  const all = tasks(catalog).slice(0, limit);
  const byId = new Map(catalog.tools.map((t) => [t.id, t]));
  let top1 = 0;
  let top3 = 0;
  let answered = 0;
  let tokens = 0;
  const misses = [];
  const wrong = [];

  for (const task of all) {
    const search = await handlers.geoprims_search({ query: task.query, limit: 3, includeExperimental: true });
    tokens += tokensOf(search);
    const ranked = search.result?.results ?? [];
    const rank = ranked.findIndex((r) => r.id === task.id);
    if (rank === 0) top1 += 1;
    if (rank >= 0) top3 += 1;
    else {
      misses.push({ query: task.query, wanted: task.id, got: ranked[0]?.id ?? null });
      continue;
    }

    // What an agent would do next: run the tool it found, with what it filled.
    const chosen = ranked[0];
    const args = chosen.prefill ?? {};
    const got = await handlers.geoprims_run({ id: chosen.id, args: Object.keys(args).length ? args : undefined });
    tokens += tokensOf(got);
    const want = await handlers.geoprims_run({ id: task.id });
    if (chosen.id === task.id && JSON.stringify(got.result?.result) === JSON.stringify(want.result?.result)) {
      answered += 1;
    } else if (chosen.id === task.id) {
      wrong.push({ query: task.query, id: task.id, note: 'the filled arguments gave a different answer than the worked example' });
    }
  }

  const pct = (n) => Math.round((n / all.length) * 1000) / 10;
  return {
    tasks: all.length,
    domains: new Set(all.map((t) => t.id.split('.')[0])).size,
    selection1: pct(top1),
    selection3: pct(top3),
    answered: pct(answered),
    tokensPerTask: Math.round(tokens / all.length),
    misses: misses.slice(0, 20),
    wrong: wrong.slice(0, 20),
    unknown: all.filter((t) => !byId.has(t.id)).map((t) => t.id),
  };
}

/** Builds the handlers an evaluation runs against, from a local build. */
export async function localHandlers(root) {
  const { readFileSync } = await import('node:fs');
  const { join } = await import('node:path');
  const { workerHost } = await import('../../packages/runtime/src/worker-host.mjs');
  const { metaHandlers } = await import('../../mcp/meta.mjs');
  const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
  const host = workerHost(join(root, 'dist/wasm'), { timeoutMs: 15_000, maxBytes: 10_000_000 });
  await host.searchLoad(JSON.stringify(catalog.tools));
  const modules = JSON.parse(readFileSync(join(root, 'dist/wasm/modules.json'), 'utf8')).modules;
  const limits = JSON.parse(readFileSync(join(root, 'data/report-limits.json'), 'utf8'));
  return { catalog, host, handlers: metaHandlers({ host, catalog, modules, limits }) };
}

// `node tools/mcp/eval.mjs` prints the report; `--write` records the baseline.
if (import.meta.url === `file://${process.argv[1]}`) {
  const { writeFileSync } = await import('node:fs');
  const { join } = await import('node:path');
  const root = new URL('../..', import.meta.url).pathname;
  const { catalog, host, handlers } = await localHandlers(root);
  const r = await evaluate({ catalog, handlers });
  const head = {
    tasks: r.tasks,
    domains: r.domains,
    selection1: r.selection1,
    selection3: r.selection3,
    answered: r.answered,
    tokensPerTask: r.tokensPerTask,
  };
  console.log(JSON.stringify(head, null, 2));
  if (r.misses.length) console.log(`\nfirst misses:\n${r.misses.map((m) => `  ${m.query} → ${m.got ?? 'nothing'} (wanted ${m.wanted})`).join('\n')}`);
  if (process.argv.includes('--write')) {
    writeFileSync(join(root, 'data/mcp-eval.json'), JSON.stringify({ ...head, misses: r.misses, wrong: r.wrong }, null, 2) + '\n');
    console.log('\nwrote data/mcp-eval.json');
  }
  await host.close?.();
  process.exit(0);
}
