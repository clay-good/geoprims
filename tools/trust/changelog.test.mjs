import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { checkChangelog, readChangelog, supersededVectors } from './changelog.mjs';

const root = new URL('../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const today = new Date().toISOString().slice(0, 10);

test('the changelog records every superseded vector as a result change', () => {
  const superseded = supersededVectors(root);
  assert.ok(superseded.length >= 10);
  assert.deepEqual(checkChangelog(readChangelog(root), { catalog, superseded, today }), []);
});

test('a superseded vector without an entry fails the gate', () => {
  const log = readChangelog(root);
  const superseded = [...supersededVectors(root), 'time.sun.position v999'];
  assert.deepEqual(checkChangelog(log, { catalog, superseded, today }), ['superseded vector time.sun.position v999 has no result-change entry']);
  const future = { entries: [{ date: '2999-01-01', kind: 'added', tools: [], vectors: [], summary: 'x' }] };
  assert.deepEqual(checkChangelog(future, { catalog, superseded: [], today }), ['entry 1 (2999-01-01): dated in the future']);
});
