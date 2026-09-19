// The ranking and prefill accuracy gate: fails when any metric drops more
// than 2 points below the recorded baseline, listing the questions that
// changed. UPDATE_ACCURACY=1 records a new baseline.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { workerHost } from '../../packages/runtime/src/worker-host.mjs';
import { generate, measure } from './accuracy.mjs';

const root = join(new URL('.', import.meta.url).pathname, '../..');
const baselinePath = join(root, 'data/search-accuracy.json');
const METRICS = ['top1', 'top3', 'prefillPrecision', 'prefillRecall'];

test('search and prefill accuracy stay within 2 points of the baseline', async () => {
  const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
  const h = workerHost(join(root, 'dist/wasm'));
  try {
    await h.searchLoad(JSON.stringify(catalog.tools));
    const cases = generate(catalog.tools);
    assert.ok(cases.length >= 500, `${cases.length} questions`);
    const now = await measure(cases, catalog.tools, (r) => h.search(r));
    if (process.env.UPDATE_ACCURACY) {
      writeFileSync(baselinePath, JSON.stringify(now, null, 2) + '\n');
      return;
    }
    const base = JSON.parse(readFileSync(baselinePath, 'utf8'));
    const dropped = METRICS.filter((k) => now[k] < base[k] - 2);
    const newMisses = now.misses.filter((q) => !base.misses.includes(q));
    const newWrong = now.wrongFills.filter((q) => !base.wrongFills.includes(q));
    assert.deepEqual(
      dropped,
      [],
      `${dropped.map((k) => `${k} ${base[k]}% → ${now[k]}%`).join(', ')}\nNo longer in the top 3:\n  ${newMisses.join('\n  ')}\nNew wrong fills:\n  ${newWrong.join('\n  ')}`,
    );
    // Prefill must not guess: a wrong fill is worse than an empty field.
    assert.ok(now.prefillPrecision >= 95, `prefill precision ${now.prefillPrecision}%:\n  ${now.wrongFills.join('\n  ')}`);
  } finally {
    h.close();
  }
});
