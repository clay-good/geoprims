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
  { name: 'how we got this', find: /<summary>How we got this/ },
  { name: 'related tools', find: /<h2 id="related-title">Related tools<\/h2>/, optional: true },
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
    if (!/>Related tools<\/h2>/.test(html) && !/class="other-way"/.test(html)) {
      problems.push(`${t.id}: a stable tool points at no related tool`);
    }
  }
  assert.deepEqual(problems, []);
});

test('exactly one report button in the tool, in the answer card (the footer has its own)', () => {
  for (const t of catalog.tools) {
    const html = page(route(t.id));
    const buttons = [...html.slice(0, html.indexOf('<footer class="site">')).matchAll(/>Report a problem</g)];
    assert.equal(buttons.length, 1, `${t.id} has ${buttons.length} report buttons`);
    const answer = /<section class="card answer"[\s\S]*?<\/section>/.exec(html)?.[0] ?? '';
    assert.match(answer, />Report a problem</, `${t.id}: the report button is not in the answer card`);
  }
});

test('the gate catches a page whose regions are out of order', () => {
  const html = page(route('aviation.altimetry.density-altitude'));
  assert.deepEqual(anatomyProblems(html), []);
  // Move the related tools above "How we got this".
  const related = /<section class="related"[\s\S]*?<\/section>/.exec(html)[0];
  const howWeGot = /<details class="card proof">/.exec(html)[0];
  const moved = html.replace(related, '').replace(howWeGot, related + howWeGot);
  assert.deepEqual(anatomyProblems(moved), ['related tools comes before how we got this']);
});

test('a tool page is the tool, its proof, and nothing for developers', () => {
  // The owner's call: tools, a worked example, Report a problem, and one
  // panel of explanation and proof. The agent material lives on /agents/.
  const problems = [];
  for (const t of catalog.tools) {
    const html = page(route(t.id));
    if (/For developers and agents|class="card dev"|geoprims_run/.test(html.replace(/<script[\s\S]*?<\/script>/g, ''))) {
      problems.push(`${t.id}: still carries developer material`);
    }
    if ((html.match(/<details class="card proof">/g) ?? []).length !== 1) problems.push(`${t.id}: not exactly one proof panel`);
  }
  assert.deepEqual(problems, []);
});

test('the gate catches a page with no answer card', () => {
  const html = page(route('aviation.altimetry.density-altitude'));
  assert.deepEqual(anatomyProblems(html.replace('<section class="card answer"', '<section class="card result"')), ['no answer card']);
});

/**
 * Answer first on a phone (ux/glanceable-results). Without a browser this is
 * measured as the text a phone would have to scroll past before the answer:
 * at 390 px a line of body text holds about 45 characters and stands about
 * 24 px tall, so a budget of 1,200 characters is about 640 px — the header,
 * the card's own padding, and the value itself still inside an 844 px screen.
 * It is a proxy for the layout, not the layout itself.
 */
const PHONE_BUDGET = 1200;

/** What a phone renders above a point: no scripts, no noscript, no closed details. */
const visibleText = (html) =>
  html
    .replace(/<script[\s\S]*?<\/script>/g, '')
    .replace(/<style[\s\S]*?<\/style>/g, '')
    .replace(/<noscript>[\s\S]*?<\/noscript>/g, '')
    .replace(/<details(?![^>]*\bopen\b)[^>]*>[\s\S]*?<\/details>/g, '')
    .replace(/<[^>]+>/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();

test('the answer is the first thing on a tool page, not the inputs', () => {
  const problems = [];
  for (const t of catalog.tools) {
    const html = page(route(t.id));
    const answer = html.indexOf('<section class="card answer"');
    const inputs = html.indexOf('<form class="card inputs"');
    if (!(answer > 0 && answer < inputs)) problems.push(`${t.id}: the inputs come first`);
  }
  assert.deepEqual(problems, []);
});

test('little enough stands above the answer for a phone to show it', () => {
  const over = [];
  for (const t of catalog.tools) {
    const html = page(route(t.id));
    const above = visibleText(html.slice(html.indexOf('<main'), html.indexOf('<section class="card answer"')));
    if (above.length > PHONE_BUDGET) over.push(`${t.id}: ${above.length} characters above the answer`);
  }
  assert.deepEqual(over, []);
});

test('the phone budget is measured, not assumed', () => {
  // A page with a wall of text above the answer fails, so the budget bites.
  const html = page(route('units.length.convert'));
  const bloated = html.replace('<section class="card answer"', `<p>${'word '.repeat(300)}</p><section class="card answer"`);
  const above = visibleText(bloated.slice(bloated.indexOf('<main'), bloated.indexOf('<section class="card answer"')));
  assert.ok(above.length > PHONE_BUDGET);
});
