import assert from 'node:assert/strict';
import { test } from 'node:test';
import { buildDate } from '../src/lib/build-date.mjs';

test('a build date comes from the commit timestamp across a UTC day boundary', () => {
  assert.equal(buildDate('1789948799'), '2026-09-20');
  assert.equal(buildDate('1789948800'), '2026-09-21');
  assert.throws(() => buildDate('later'), /SOURCE_DATE_EPOCH/);
});
