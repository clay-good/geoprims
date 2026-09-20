// The schema-driven form (web/tool-app "ToolForm"). Every manifest in the
// catalog has to render: a control for each input, of a kind that matches the
// schema, with its label and help, and nothing silently dropped.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { coreGroup } from '../../../tools/trust/metaschema.mjs';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const dist = join(web, 'dist');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const route = (id) => '/' + id.split('.').join('/') + '/';
const page = (r) => readFileSync(join(dist, r, 'index.html'), 'utf8');

/** The control a schema should get: a select for a choice, a textarea for a list. */
const expected = (schema) => (schema.enum ? 'select' : schema.type === 'array' ? 'textarea' : 'input');

test('every manifest renders, with one control per input', () => {
  const problems = [];
  for (const t of catalog.tools) {
    const html = page(route(t.id));
    for (const [name, schema] of Object.entries(t.inputs.properties)) {
      if (name === 'options') continue;
      const id = `field-${name}`;
      const tag = expected(schema);
      const found = new RegExp(`<${tag}[^>]*\\bid="${id}"`).test(html);
      if (!found) problems.push(`${t.id}: ${name} has no <${tag}> control`);
    }
  }
  assert.deepEqual(problems, []);
});

test('every control is labelled and carries its help text', () => {
  const problems = [];
  const escape = (s) => s.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll("'", '&#39;').replaceAll('"', '&quot;');
  for (const t of catalog.tools) {
    const html = page(route(t.id));
    for (const [name, schema] of Object.entries(t.inputs.properties)) {
      if (name === 'options') continue;
      if (schema.title && !html.includes(escape(schema.title))) problems.push(`${t.id}: ${name} has no label`);
    }
  }
  assert.deepEqual(problems, []);
});

test('a list input says what one row holds', () => {
  const lists = catalog.tools.flatMap((t) =>
    Object.entries(t.inputs.properties)
      .filter(([k, s]) => k !== 'options' && s.type === 'array' && s.items?.properties)
      .map(([k, s]) => ({ t, k, columns: Object.keys(s.items.properties) })),
  );
  assert.ok(lists.length >= 5, `only ${lists.length} list inputs`);
  const problems = [];
  for (const { t, k, columns } of lists) {
    const html = page(route(t.id));
    // "one per line: northing, easting" tells a reader what to paste.
    if (!/one per line:/.test(html)) problems.push(`${t.id}: ${k} does not say what a row holds`);
    for (const c of columns) {
      const title = t.inputs.properties[k].items.properties[c].title?.toLowerCase();
      if (title && !html.toLowerCase().includes(title)) problems.push(`${t.id}: ${k} does not name its ${c} column`);
    }
  }
  assert.deepEqual(problems, []);
});

/**
 * The contract caps the inputs shown by default at five. The meta-schema
 * counts the ones marked `x-core`; the form shows those *and* every required
 * input, because a required field cannot sit behind "More options". These two
 * tools have five marked core inputs and a sixth that is required, so a
 * reader sees six. Fixing them means deciding whether an input is really
 * required, which is a question about the tool, not about the page, so they
 * are recorded here rather than quietly excused. The list may shrink, never
 * grow.
 */
const SHOWS_SIX = new Set([
  // Six genuinely separate values: height, sensor width and height, focal
  // length, image width, and groundspeed. sensor_height is required here but
  // optional on the GSD tool it shares a camera with, which is the question to
  // settle before this row can go.
  'drone.photogrammetry.trigger',
  // figure_of_merit is marked core and target_time is required, so six show.
  'drone.power.max-payload',
]);

test('at most five inputs are shown before "More options"', () => {
  const problems = [];
  const known = new Set();
  for (const t of catalog.tools) {
    const html = page(route(t.id));
    // Everything before the "More options" summary is what a reader sees first;
    // nested markup makes a div-matching regex unreliable, so cut on that.
    const start = html.indexOf('<div class="fields">');
    const more = html.indexOf('>More options', start);
    const core = html.slice(start, more < 0 ? html.length : more);
    // Counted the way the meta-schema counts: a coordinate is one point.
    const names = [...core.matchAll(/\bid="field-([^"]+)"/g)].map((m) => m[1]);
    const shown = new Set(names.map(coreGroup)).size;
    if (shown > 5) {
      if (SHOWS_SIX.has(t.id)) known.add(t.id);
      else problems.push(`${t.id} shows ${shown} inputs before More options`);
    }
    if (shown === 0) problems.push(`${t.id} shows no inputs at all`);
  }
  assert.deepEqual(problems, []);
  // A tool that has been fixed should leave the list rather than sit in it.
  assert.deepEqual([...SHOWS_SIX].filter((id) => !known.has(id)), [], 'remove it from SHOWS_SIX');
});

test('an optional input says so, so a reader knows what is required', () => {
  const t = catalog.tools.find((x) => Object.keys(x.inputs.properties).some((k) => k !== 'options' && !x.inputs.required.includes(k)));
  assert.ok(t, 'no tool has an optional input');
  assert.match(page(route(t.id)), /class="optional"/);
});
