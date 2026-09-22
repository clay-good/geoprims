// What a reader sees names no file in this repository. A citation once read
// "current as of the review date in data/regulations.json": true for a
// maintainer, meaningless on a tool page. Every reader-facing string in the
// catalog — titles, help, model, accuracy, citations, limitations — is
// checked; links are not, since an external document's path is its address.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const root = join(new URL('.', import.meta.url).pathname, '../..');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const REPO_PATH = /\b(data|docs|core|apps|tools|packages|openspec|mcp|worker)\/[\w.-]+|\.(jsonl?|rs|mjs|astro|svelte)\b/;

/** Every string in a value, with its path, skipping links and machine fields. */
function* strings(v, path = '') {
  if (typeof v === 'string') yield [path, v];
  else if (v && typeof v === 'object') {
    for (const [k, x] of Object.entries(v)) {
      if (['url', 'vectors', 'examples', 'freeAccessUrl', '$schema', 'id'].includes(k)) continue;
      yield* strings(x, path ? `${path}.${k}` : k);
    }
  }
}

test('no reader-facing text names a file in this repository', () => {
  const problems = [];
  for (const t of catalog.tools) {
    for (const [path, text] of strings(t)) {
      if (REPO_PATH.test(text)) problems.push(`${t.id} ${path}: ${text.slice(0, 80)}`);
    }
  }
  assert.deepEqual(problems, []);
});

test('the check sees a leaked path', () => {
  assert.ok(REPO_PATH.test('current as of the review date in data/regulations.json'));
  assert.ok(!REPO_PATH.test('14 CFR Part 107, eCFR, current text'));
});
