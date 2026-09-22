// Every tool answers in a sentence (glanceable-and-field-ux: the answer as a
// sentence). A tool whose sentence template cannot render returns a result
// with no summary at all, which is a silent hole: the page and the agent both
// lead with that line. The per-crate manifest lint catches a bad template, but
// only for crates that run it, so this checks the built catalog instead.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { nodeHost } from '../../packages/runtime/src/node.mjs';

const root = new URL('../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const host = nodeHost(join(root, 'dist/wasm'));
const primary = (t) => t.examples.find((e) => e.id === t['x-primary-example']) ?? t.examples[0];

test('every tool renders a summary for its worked example', async () => {
  const problems = [];
  for (const t of catalog.tools) {
    const ex = primary(t);
    if (!ex) continue;
    const r = JSON.parse(await host.invoke(t.id, JSON.stringify(ex.input)));
    if (!r.ok) {
      problems.push(`${t.id}: its worked example does not run`);
      continue;
    }
    if (typeof r.summary !== 'string' || !r.summary.trim()) problems.push(`${t.id}: no summary`);
    // A template that rendered nothing for a field leaves the braces or an
    // empty gap behind, which reads as a broken sentence.
    else if (/[{}]/.test(r.summary)) problems.push(`${t.id}: its summary still has template braces: ${r.summary}`);
  }
  assert.deepEqual(problems, []);
});
