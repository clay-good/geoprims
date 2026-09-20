// The canonical tool-page anatomy (contracts/page-chrome). Every tool page
// lays its regions out in one fixed order, so a reader who learns one page has
// learned them all, and the answer is always in the same place.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const dist = join(web, 'dist');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const route = (id) => '/' + id.split('.').join('/') + '/';
const page = (r) => readFileSync(join(dist, r, 'index.html'), 'utf8');

/**
 * The contract's regions, in order, each with the markup that identifies it.
 * `optional` regions appear only on the pages that need them; the rest must be
 * on every tool page.
 */
export const REGIONS = [
  { name: 'title block', find: /<h1[ >]/, },
  { name: 'notices', find: /role="note"/, optional: true },
  { name: 'answer card', find: /<section class="card answer"/ },
  { name: 'core inputs', find: /<form class="card inputs"/ },
  { name: 'more options', find: /<summary>More options/, optional: true },
  { name: 'canvas', find: /<section class="card canvas"/, optional: true },
  { name: 'how we got this', find: /<summary>How we got this<\/summary>/ },
  { name: 'terms on this page', find: /<summary>Terms on this page<\/summary>/, optional: true },
  { name: 'related tools', find: /<h2>Related tools<\/h2>/, optional: true },
  { name: 'for developers and agents', find: /<summary>For developers and agents<\/summary>/ },
];

/** Every region missing or out of order on one page. */
export function anatomyProblems(html) {
  const problems = [];
  let last = -1;
  let lastName = 'the start of the page';
  for (const r of REGIONS) {
    const at = html.search(r.find);
    if (at < 0) {
      if (!r.optional) problems.push(`no ${r.name}`);
      continue;
    }
    if (at < last) problems.push(`${r.name} comes before ${lastName}`);
    last = at;
    lastName = r.name;
  }
  return problems;
}

test('every tool page lays out the contract regions in order', () => {
  const problems = [];
  for (const t of catalog.tools) {
    for (const p of anatomyProblems(page(route(t.id)))) problems.push(`${t.id}: ${p}`);
  }
  assert.deepEqual(problems, []);
});

test('every stable tool page carries all the regions the contract requires', () => {
  const problems = [];
  for (const t of catalog.tools.filter((x) => x.stability === 'stable' && x.composedOf.length === 0)) {
    const html = page(route(t.id));
    for (const r of REGIONS.filter((x) => !x.optional)) {
      if (html.search(r.find) < 0) problems.push(`${t.id}: no ${r.name}`);
    }
    // A tool whose only relation is its inverse shows it as the "Go the other
    // way" link in the title block instead of a section of one.
    if (!/<h2>Related tools<\/h2>/.test(html) && !/class="other-way"/.test(html)) {
      problems.push(`${t.id}: a stable tool points at no related tool`);
    }
  }
  assert.deepEqual(problems, []);
});

test('exactly one report button per page, in the answer card', () => {
  for (const t of catalog.tools) {
    const html = page(route(t.id));
    const buttons = [...html.matchAll(/>Report a problem</g)];
    assert.equal(buttons.length, 1, `${t.id} has ${buttons.length} report buttons`);
    const answer = /<section class="card answer"[\s\S]*?<\/section>/.exec(html)?.[0] ?? '';
    assert.match(answer, />Report a problem</, `${t.id}: the report button is not in the answer card`);
  }
});

test('the gate catches a page whose regions are out of order', () => {
  const html = page(route('aviation.altimetry.density-altitude'));
  assert.deepEqual(anatomyProblems(html), []);
  // Move the developer block above "How we got this".
  const dev = /<details class="card dev">[\s\S]*?<\/details>/.exec(html)[0];
  const howWeGot = /<details[^>]*>\s*<summary>How we got this<\/summary>/.exec(html)[0];
  const moved = html.replace(dev, '').replace(howWeGot, dev + howWeGot);
  assert.deepEqual(anatomyProblems(moved), ['for developers and agents comes before related tools']);
});

test('the gate catches a page with no answer card', () => {
  const html = page(route('aviation.altimetry.density-altitude'));
  assert.deepEqual(anatomyProblems(html.replace('<section class="card answer"', '<section class="card result"')), ['no answer card']);
});
