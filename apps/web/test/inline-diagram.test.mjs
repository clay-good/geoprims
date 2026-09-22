// The compact diagram under the answer (ux/glanceable-results, "Fixed
// tool-page anatomy"). For a tool whose meaning is a picture, the picture
// belongs beside the number, not below the inputs — the full canvas stays in
// its anatomy position.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { DIAGRAM_TOOLS, diagram } from '../src/lib/diagrams.js';
import { nodeHost } from '../../../packages/runtime/src/node.mjs';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const page = (id) => readFileSync(join(web, 'dist', ...id.split('.'), 'index.html'), 'utf8');
const inlineTools = catalog.tools.filter((t) => t['x-diagram-inline']).map((t) => t.id);
const host = nodeHost(join(root, 'dist/wasm'));
/** A tool's primary example, run for real. */
async function ran(id) {
  const t = catalog.tools.find((x) => x.id === id);
  const args = (t.examples.find((e) => e.id === t['x-primary-example']) ?? t.examples[0]).input;
  return [args, JSON.parse(await host.invoke(id, JSON.stringify(args)))];
}

test('every tool whose meaning is a picture has a drawing', () => {
  // Other tools may draw a supporting diagram (a profile, a dial) in the canvas
  // position without being pictures; the reverse must always hold.
  assert.ok(inlineTools.length >= 5);
  for (const id of inlineTools) assert.ok(DIAGRAM_TOOLS.includes(id), `${id}: flagged x-diagram-inline but draws nothing`);
});

test('each of those pages draws the picture under the answer, and again as the canvas', () => {
  for (const id of inlineTools) {
    const html = page(id);
    const inline = html.indexOf('class="inline-diagram"');
    const form = html.indexOf('<form class="card inputs"');
    const canvas = html.indexOf('<figure class="diagram card">');
    assert.ok(inline > 0, `${id}: no compact diagram`);
    assert.ok(inline < form, `${id}: the compact diagram is not above the inputs`);
    assert.ok(canvas > form, `${id}: the full canvas is not in its anatomy position`);
  }
});

test('a tool that is not a picture gets no compact diagram', () => {
  for (const id of ['aviation.altimetry.density-altitude', 'units.length.convert']) {
    assert.ok(!page(id).includes('inline-diagram'), id);
  }
});

test('the two drawings do not share an id', () => {
  for (const id of inlineTools) {
    const ids = [...page(id).matchAll(/<(?:marker|linearGradient|clipPath|symbol)[^>]*\bid="([^"]+)"/g)].map((m) => m[1]);
    assert.deepEqual(ids.length, new Set(ids).size, `${id}: ${ids.join(', ')}`);
  }
});

test('the compact copy is the same picture, scoped', async () => {
  for (const id of inlineTools) {
    const [args, result] = await ran(id);
    const full = diagram(id, args, result);
    const compact = diagram(id, args, result, 'inline');
    assert.ok(full && compact, `${id}: the diagram did not draw`);
    assert.equal(compact.desc, full.desc);
    assert.equal(compact.markup.replaceAll('-inline"', '"').replaceAll('-inline)', ')'), full.markup, id);
  }
});

test('a drawing after a scoped one is unscoped again', async () => {
  const id = 'aviation.wind.runway-components';
  const [args, result] = await ran(id);
  diagram(id, args, result, 'inline');
  const after = diagram(id, args, result);
  assert.ok(after && !after.markup.includes('-inline'), 'the scope leaked into the next drawing');
});
