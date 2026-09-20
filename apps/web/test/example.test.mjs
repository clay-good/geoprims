// Prefilled examples (ux/glanceable-results, "Prefilled examples and clear
// controls"). A tool opened cold shows a real answer computed from its worked
// example, says that is what it is, and offers both ways out: Clear empties
// every field, Try the example puts them back.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const page = (id) => readFileSync(join(web, 'dist', ...id.split('.'), 'index.html'), 'utf8');
const app = readFileSync(join(web, 'src/components/ToolApp.svelte'), 'utf8');

test('a tool opened cold is filled with its example and says so', () => {
  const problems = [];
  for (const t of catalog.tools) {
    const html = page(t.id);
    if (!html.includes('Showing an example. Change anything.')) problems.push(`${t.id}: no example label`);
    // A value sits in an input's value, in a select's chosen option, or as
    // the rows inside a list's textarea.
    const filled =
      [...html.matchAll(/<input[^>]*\bid="field-[^"]*"[^>]*\bvalue="([^"]*)"/g)].some((m) => m[1] !== '') ||
      [...html.matchAll(/<textarea[^>]*\bid="field-[^"]*"[^>]*>([\s\S]*?)<\/textarea>/g)].some((m) => m[1].trim() !== '') ||
      html.includes('<select id="field-');
    const inputs = Object.keys(t.inputs.properties).filter((k) => k !== 'options');
    if (inputs.length && !filled) problems.push(`${t.id}: no field carries an example value`);
  }
  assert.deepEqual(problems, []);
});

test('the answer is in the HTML, before any script runs', () => {
  // The GSD example the requirement names.
  const html = page('drone.photogrammetry.gsd');
  const value = /class="value"[^>]*>([^<]*)/.exec(html)?.[1];
  assert.equal(value, '2.741');
  assert.match(html, /cm\/px/);
});

test('both ways out are on the page', () => {
  for (const id of ['drone.photogrammetry.gsd', 'aviation.altimetry.density-altitude']) {
    const html = page(id);
    assert.match(html, />Try the example</, id);
    assert.match(html, />Clear</, id);
  }
});

test('Clear empties every field, and the example label goes with it', () => {
  // Clearing writes an empty string to each field, so nothing is left behind.
  assert.match(app, /function clearAll\(\)[\s\S]*?values\[k\] = ''/);
  assert.match(app, /isExample = false/);
});

test('editing a value drops the example label', () => {
  assert.match(app, /function edited\(\)[\s\S]*?isExample = false/);
});

test('the label is shown only while the values are the example', () => {
  assert.match(app, /\{#if isExample\}<span class="chip">Showing an example\. Change anything\.<\/span>\{\/if\}/);
});
