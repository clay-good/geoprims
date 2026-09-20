// Per-invocation limits (platform/compute-core, "Resource limits"). A call
// that would be too large is refused before the tool does any work, with
// LIMIT_EXCEEDED naming the field and the cap, and the same call one row
// under the cap still succeeds.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { nodeHost } from './node.mjs';

const root = new URL('../../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const host = nodeHost(join(root, 'dist/wasm'));
const primary = (t) => t.examples.find((e) => e.id === t['x-primary-example']) ?? t.examples[0];

/** Every capped list input a tool's own example gives rows for. */
function cappedLists() {
  const out = [];
  for (const t of catalog.tools) {
    const input = primary(t)?.input ?? {};
    for (const [field, schema] of Object.entries(t.inputs.properties)) {
      if (schema.type === 'array' && schema.maxItems && Array.isArray(input[field]) && input[field].length > 0) {
        out.push({ tool: t, field, cap: schema.maxItems, rows: input[field], input });
      }
    }
  }
  return out;
}

/** The example's rows, repeated to exactly `n` of them. */
const repeat = (rows, n) => Array.from({ length: n }, (_, i) => rows[i % rows.length]);

test('every capped list refuses one row over its cap, naming the field', async () => {
  const cases = cappedLists();
  assert.ok(cases.length >= 10, `only ${cases.length} capped lists`);
  const problems = [];
  for (const { tool, field, cap, rows, input } of cases) {
    const over = JSON.parse(await host.invoke(tool.id, JSON.stringify({ ...input, [field]: repeat(rows, cap + 1) })));
    if (over.ok) {
      problems.push(`${tool.id}/${field}: ${cap + 1} rows was accepted, the cap is ${cap}`);
      continue;
    }
    if (over.error.code !== 'LIMIT_EXCEEDED') {
      problems.push(`${tool.id}/${field}: ${cap + 1} rows gave ${over.error.code}, not LIMIT_EXCEEDED`);
      continue;
    }
    // The refusal says what was too big and what the cap is, so it can be acted on.
    if (!String(over.error.message).includes(String(cap))) {
      problems.push(`${tool.id}/${field}: the message does not name the cap ${cap}: ${over.error.message}`);
    }
    // A pre-check: nothing was computed.
    if (over.result !== undefined) problems.push(`${tool.id}/${field}: a refused call still returned a result`);
  }
  assert.deepEqual(problems, []);
});

test('the cap itself is not off by one', async () => {
  // The biggest list a tool will take must still work. Checked on the tools
  // whose caps are small enough to build quickly.
  const cases = cappedLists().filter((c) => c.cap <= 2000);
  assert.ok(cases.length > 0);
  const problems = [];
  for (const { tool, field, cap, rows, input } of cases) {
    const at = JSON.parse(await host.invoke(tool.id, JSON.stringify({ ...input, [field]: repeat(rows, cap) })));
    // Repeating a row can be geometrically degenerate; only a limit error is wrong here.
    if (!at.ok && at.error.code === 'LIMIT_EXCEEDED') {
      problems.push(`${tool.id}/${field}: ${cap} rows was refused, but the cap is ${cap}`);
    }
  }
  assert.deepEqual(problems, []);
});

test('an oversized payload is refused before it is parsed', async () => {
  const { checkJson } = await import('./harden.mjs');
  const big = JSON.stringify({ s: 'x'.repeat(2_000_000) });
  const r = checkJson(big);
  assert.equal(r.error.code, 'LIMIT_EXCEEDED');
  assert.match(r.error.message, /bytes/);
  assert.equal(r.result, undefined, 'nothing was parsed');
});
