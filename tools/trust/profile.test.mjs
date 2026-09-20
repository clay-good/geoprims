// The reference profile (contracts/reference-profiles) is one file, and the
// documentation, the contract, and any gate that reports a budget all read it
// rather than repeating its numbers.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const root = new URL('../..', import.meta.url).pathname;
const profile = JSON.parse(readFileSync(join(root, 'data/reference-profile.json'), 'utf8'));
const doc = readFileSync(join(root, 'docs/performance.md'), 'utf8');

test('the profile is versioned and dated', () => {
  assert.match(profile.version, /^\d+\.\d+\.\d+$/);
  assert.match(profile.since, /^\d{4}-\d{2}-\d{2}$/);
  assert.ok(doc.includes(`Current profile: ${profile.version}`), 'docs/performance.md names another version');
  assert.ok(doc.includes(`| ${profile.version} | ${profile.since} |`), 'the version is not in the history table');
});

test('the documented viewports are the ones the gates would use', () => {
  assert.equal(profile.viewports.length, 6);
  for (const v of profile.viewports) {
    assert.ok(doc.includes(`${v.width} × ${v.height}`), `${v.name} is not documented`);
  }
  // The answer-above-the-fold check has exactly one viewport.
  const fold = profile.viewports.filter((v) => v.aboveTheFold);
  assert.equal(fold.length, 1);
  assert.deepEqual([fold[0].width, fold[0].height], [390, 844]);
  assert.equal(profile.textZoom.percent, 200);
  assert.equal(profile.textZoom.width, 375);
});

test('every budget has a hard number, and targets are stricter than hard limits', () => {
  for (const [name, b] of Object.entries(profile.budgets)) {
    assert.ok(Number.isFinite(b.hard), `${name} has no hard limit`);
    if (b.target !== undefined) assert.ok(b.target < b.hard, `${name}'s target is not stricter than its hard limit`);
    assert.ok(b.documentedAs, `${name} does not say how it is documented`);
    assert.ok(doc.includes(`| ${b.documentedAs} |`), `${name} has no row in docs/performance.md`);
  }
});

test('the profile says how it is measured, so a report can name it', () => {
  assert.equal(profile.cpu.slowdown, 4);
  assert.equal(profile.measurement.runsPerRoute, 3);
  assert.equal(profile.measurement.statistic, 'median');
  assert.deepEqual(profile.engines, ['chromium', 'webkit', 'firefox']);
  assert.ok(profile.network.downKbps > 0 && profile.network.rttMs > 0);
});

test('docs/performance.md is honest about what is measured today', () => {
  // The browser budgets have no gate yet; the page must say so rather than imply one.
  assert.match(doc, /Playwright suites are not built yet/);
  assert.match(doc, /What is measured today/);
});
