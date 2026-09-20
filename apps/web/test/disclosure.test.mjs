// Progressive disclosure with visible defaults (ux/glanceable-results). Five
// inputs at most are shown first — `test/form.test.mjs` holds that — and
// whatever the tool assumed in place of the inputs it did not get has to be
// said in words on the page, so no assumption is invisible.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const page = (id) => readFileSync(join(web, 'dist', ...id.split('.'), 'index.html'), 'utf8');
const banner = (html) => /<p class="card notice limitation" role="note">([\s\S]*?)<\/p>/.exec(html)?.[1].replace(/<[^>]+>/g, '');

test('density altitude without a dew point says it assumed dry air', () => {
  const t = catalog.tools.find((x) => x.id === 'aviation.altimetry.density-altitude');
  // The example runs on elevation, altimeter setting, and temperature only.
  assert.ok(!t.examples[0].input.dew_point, 'the example already gives a dew point');
  const text = banner(page(t.id));
  assert.ok(text, 'no assumption stated on the page');
  assert.match(text, /dry air/);
  // And it says what to do about it, naming the input that changes the answer.
  assert.match(text, /dew point/);
  assert.ok('dew_point' in t.inputs.properties, 'the remedy names an input the tool does not have');
});

test('every stated assumption is on the page it belongs to', () => {
  const problems = [];
  for (const t of catalog.tools) {
    const declared = t['x-limitation'];
    const shown = banner(page(t.id));
    if (declared && !shown) problems.push(`${t.id}: declares a limitation it does not show`);
    if (!declared && shown) problems.push(`${t.id}: shows a limitation it does not declare`);
    if (declared && shown && !shown.includes(declared.simplification)) {
      problems.push(`${t.id}: the banner does not say what the manifest says`);
    }
  }
  assert.deepEqual(problems, []);
});

test('an assumption that points at an input points at a real one', () => {
  const problems = [];
  let checked = 0;
  for (const t of catalog.tools) {
    const instead = t['x-limitation']?.instead;
    if (!instead) continue;
    // "Add the dew point and …" — the words after Add name a field of this tool.
    const asked = /\bAdd (?:the |a |an )?([a-z][a-z ]+?)(?:[,.]| and\b| to\b)/.exec(instead)?.[1];
    if (!asked) continue;
    checked += 1;
    const titles = Object.values(t.inputs.properties).map((f) => (f.title ?? '').toLowerCase());
    const names = Object.keys(t.inputs.properties).map((k) => k.replace(/_/g, ' '));
    const known = [...titles, ...names].some((n) => n.includes(asked) || asked.includes(n));
    if (!known) problems.push(`${t.id}: "Add ${asked}" is not an input of this tool`);
  }
  assert.deepEqual(problems, []);
  // Only one tool words its remedy this way today; the check is here so the
  // next one cannot point at a field that does not exist.
  assert.ok(checked >= 1, 'no limitation names an input to add');
});

test('the inputs behind "More options" are the ones a tool can do without', () => {
  const problems = [];
  for (const t of catalog.tools) {
    const html = page(t.id);
    const more = html.indexOf('>More options');
    if (more === -1) continue;
    const hidden = [...html.slice(more).matchAll(/\bid="field-([^"]+)"/g)].map((m) => m[1]);
    for (const name of hidden) {
      if ((t.inputs.required ?? []).includes(name)) problems.push(`${t.id}: ${name} is required but hidden`);
    }
  }
  assert.deepEqual(problems, []);
});
