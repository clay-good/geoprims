// Related lists resolve, give a recognised reason, and agree about inverses.
// The count is a ratchet: the number of stable tools still short of three
// related tools may fall but never rise.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { MIN_RELATED, relatedProblems, short } from './related.mjs';

const root = new URL('../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));

/**
 * How many stable tools were still short when the gate landed. Lower this
 * number as lists are curated; the test fails if it ever needs raising.
 */
const SHORT_BASELINE = 68;

test('every related list resolves, gives a reason, and agrees about inverses', () => {
  assert.deepEqual(relatedProblems(catalog), []);
});

test('the gate catches a broken list', () => {
  const t = { id: 'a.b.c', stability: 'stable', composedOf: [], related: [
    { id: 'a.b.c', reason: 'next' },
    { id: 'no.such.tool', reason: 'next' },
    { id: 'a.b.d', reason: 'sideways' },
    { id: 'a.b.d', reason: 'next' },
  ] };
  const d = { id: 'a.b.d', stability: 'stable', composedOf: [], related: [] };
  assert.deepEqual(relatedProblems({ tools: [t, d] }), [
    'a.b.c: points at itself',
    'a.b.c: related no.such.tool is not a tool',
    'a.b.c: reason sideways for a.b.d is not one of inverse, next, alternative, parent',
    'a.b.c: lists a.b.d twice',
  ]);
});

test('an inverse that is not returned is caught', () => {
  const a = { id: 'x.y.forward', stability: 'stable', composedOf: [], related: [{ id: 'x.y.inverse', reason: 'inverse' }] };
  const b = { id: 'x.y.inverse', stability: 'stable', composedOf: [], related: [] };
  assert.deepEqual(relatedProblems({ tools: [a, b] }), ['x.y.forward: x.y.inverse is its inverse but does not say so']);
  b.related.push({ id: 'x.y.forward', reason: 'inverse' });
  assert.deepEqual(relatedProblems({ tools: [a, b] }), []);
});

test(`the number of stable tools with fewer than ${MIN_RELATED} related tools only falls`, () => {
  const still = short(catalog);
  assert.ok(
    still.length <= SHORT_BASELINE,
    `${still.length} stable tools are short of ${MIN_RELATED} related tools, up from ${SHORT_BASELINE}: ${still.slice(0, 5).join(', ')}…`,
  );
  if (still.length < SHORT_BASELINE) {
    assert.fail(`only ${still.length} stable tools are short now: lower SHORT_BASELINE to ${still.length}`);
  }
});
