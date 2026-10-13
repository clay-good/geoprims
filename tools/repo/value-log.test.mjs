import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const root = new URL('../..', import.meta.url).pathname;
const log = readFileSync(`${root}/docs/value-log.md`, 'utf8');

test('the first post-launch value entry covers every tracking-free signal and target', () => {
  for (const signal of ['Search discovery', 'Correctness', 'Developer adoption', 'Practitioner trust', 'Usability']) {
    assert.match(log, new RegExp(`\\| ${signal} \\|`), `${signal} is missing`);
  }
  for (const target of ['Every hero tool indexed', 'top-10 average position', '50 GitHub stars', 'under 7 days', '1,000 weekly search clicks', '3 independent community links', 'every hero tool reviewed']) {
    assert.match(log, new RegExp(target, 'i'), `${target} is missing`);
  }
  assert.match(log, /September 2026: launch baseline/);
  assert.match(log, /September 23–30, 2026/);
  assert.match(log, /missing measurement is recorded as unavailable, not zero/i);
  assert.doesNotMatch(log, /analytics script|tracking pixel/i);
});
