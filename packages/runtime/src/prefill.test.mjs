// discovery/natural-language-prefill: every fixture question finds its tool
// and fills exactly the expected inputs, through the same Wasm search module
// the web palette and the MCP server load.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { workerHost } from './worker-host.mjs';

const root = join(new URL('.', import.meta.url).pathname, '../../..');

test('prefill fixture: tool, filled inputs, and ambiguous values', async () => {
  const h = workerHost(join(root, 'dist/wasm'));
  try {
    await h.searchLoad(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
    const { cases } = JSON.parse(readFileSync(join(root, 'data/prefill-fixture.json'), 'utf8'));
    const wrong = [];
    for (const c of cases) {
      const out = JSON.parse(await h.search(JSON.stringify({ query: c.query, limit: 1, includeExperimental: true })));
      const top = out.result.results[0] ?? {};
      const got = { id: top.id, prefill: top.prefill, ambiguous: top.ambiguous };
      const want = { id: c.id, prefill: c.prefill, ambiguous: c.ambiguous };
      try {
        assert.deepEqual(got, want);
      } catch {
        wrong.push(`${c.query}\n  got  ${JSON.stringify(got)}\n  want ${JSON.stringify(want)}`);
      }
    }
    assert.deepEqual(wrong, [], wrong.join('\n'));
  } finally {
    h.close();
  }
});
