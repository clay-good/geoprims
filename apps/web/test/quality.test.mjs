// The monthly correctness summary (feedback/triage-and-corrections). It is
// derived only from the changelog and the known-issues file, it counts no
// visitor, and it says so where a number cannot be derived yet.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { median } from '../src/lib/quality.mjs';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const dist = join(web, 'dist');
const html = readFileSync(join(dist, 'quality/index.html'), 'utf8');
const changelog = JSON.parse(readFileSync(join(root, 'data/changelog.json'), 'utf8'));
const text = html.replace(/<[^>]+>/g, ' ').replace(/\s+/g, ' ');

test('the counts match the changelog the page is built from', () => {
  const resultChanges = changelog.entries.filter((e) => e.kind === 'result-change').length;
  const fixes = changelog.entries.filter((e) => e.kind === 'fixed').length;
  assert.match(text, new RegExp(`Result changes published ${resultChanges}\\b`));
  assert.match(text, new RegExp(`Fixes published ${fixes}\\b`));
});

test('every month that saw a change has a row, newest first', () => {
  const months = [...new Set(changelog.entries.map((e) => e.date.slice(0, 7)))].sort().reverse();
  const rows = [...html.matchAll(/<th scope="row">(\d{4}-\d{2})<\/th>/g)].map((m) => m[1]);
  assert.deepEqual(rows, months);
});

test('a figure the build cannot derive says so rather than showing a zero', () => {
  assert.match(text, /Reports received not measured yet: reporting is switched off until launch/);
  assert.match(text, /Median time to fix no confirmed defects fixed yet/);
});

test('the page promises no tracking, and adds no script of its own', () => {
  assert.match(text, /No visitor is counted, no page view is recorded, and nothing is estimated/);
  // It carries the same scripts as any other static trust page: the shell's, and nothing more.
  const scripts = (page) => [...page.matchAll(/<script[^>]*>/g)].map((m) => m[0].replace(/\.[A-Za-z0-9_-]{8}\.js/, '.js')).sort();
  const plain = readFileSync(join(dist, 'disclaimer/index.html'), 'utf8');
  assert.deepEqual(scripts(html), scripts(plain));
});

test('the median is the middle, or nothing at all', () => {
  assert.equal(median([]), null);
  assert.equal(median([7]), 7);
  assert.equal(median([9, 1, 5]), 5);
  assert.equal(median([4, 2, 8, 6]), 5, 'the mean of the middle two');
});
