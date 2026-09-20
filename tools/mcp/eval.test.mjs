// The agent-evaluation gate (add-local-mcp-server, "Agent evaluation"). The
// suite measures what an agent experiences end to end; this holds it to the
// recorded baseline, so a change that makes tools harder to find or more
// expensive to call fails the build instead of going unnoticed.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { after, test } from 'node:test';
import assert from 'node:assert/strict';
import { evaluate, localHandlers, tasks, tokensOf } from './eval.mjs';

const root = new URL('../..', import.meta.url).pathname;
const baseline = JSON.parse(readFileSync(join(root, 'data/mcp-eval.json'), 'utf8'));
const catalogFile = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));

/** The drop the gate allows before failing, in percentage points. */
const SLACK = 3;
/** How much dearer a task may get before the gate fails, as a share. */
const TOKEN_SLACK = 1.25;

let host;
after(() => host?.close?.());

test('the suite covers every domain with well over a hundred tasks', () => {
  const all = tasks(catalogFile);
  assert.ok(all.length >= 100, `only ${all.length} tasks`);
  const domains = new Set(all.map((t) => t.id.split('.')[0]));
  assert.ok(domains.size >= 9, `only ${domains.size} domains: ${[...domains].join(', ')}`);
  // Every task names a tool that exists and gives it arguments to be run with.
  const ids = new Set(catalogFile.tools.map((t) => t.id));
  for (const t of all) assert.ok(ids.has(t.id), `${t.id} is not a tool`);
});

test('selection, answers, and cost stay within the baseline', async () => {
  const local = await localHandlers(root);
  host = local.host;
  const r = await evaluate({ catalog: local.catalog, handlers: local.handlers });
  const problems = [];
  for (const k of ['selection1', 'selection3', 'answered']) {
    if (r[k] < baseline[k] - SLACK) problems.push(`${k} fell from ${baseline[k]}% to ${r[k]}%`);
  }
  if (r.tokensPerTask > baseline.tokensPerTask * TOKEN_SLACK) {
    problems.push(`a task costs ${r.tokensPerTask} tokens, up from ${baseline.tokensPerTask}`);
  }
  assert.deepEqual(problems, [], `${problems.join('; ')}. Review, then run: node tools/mcp/eval.mjs --write`);
  // A real improvement should be recorded, not left implicit.
  for (const k of ['selection1', 'selection3', 'answered']) {
    assert.ok(r[k] <= baseline[k] + SLACK, `${k} rose to ${r[k]}%: record it with node tools/mcp/eval.mjs --write`);
  }
});

test('the token estimate is the payload, not a guess', () => {
  assert.equal(tokensOf({}), 1);
  assert.equal(tokensOf('x'.repeat(40)), Math.ceil(42 / 4), 'the JSON quotes count too');
});
