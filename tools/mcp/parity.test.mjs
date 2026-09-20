// Hero-tool parity between the two surfaces (mcp/tool-surface 2.5a). A pilot
// reading the page and an agent calling the server must get the same answer,
// the same sentence, the same citations, and the same work — otherwise one of
// them is being told something the other is not.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { localHandlers } from './eval.mjs';

const root = new URL('../..', import.meta.url).pathname;
const checklist = readFileSync(join(root, 'docs/launch/hero-tools.md'), 'utf8');
/** Every tool id on the hero checklist, in the order it is listed. */
const HERO = [...new Set([...checklist.matchAll(/\|\s*`([a-z0-9.-]+)`\s*\|/g)].map((m) => m[1]))];
const page = (id) => readFileSync(join(root, 'apps/web/dist', ...id.split('.'), 'index.html'), 'utf8');
/** The answer card's big number as a reader sees it ("7,932 ft"). */
const pageAnswer = (html) =>
  (/<div class="value"[^>]*>([\s\S]*?)<\/div>/.exec(html)?.[1] ?? '')
    .replace(/<!--.*?-->/g, '')
    .replace(/<[^>]+>/g, ' ')
    .replaceAll('&#39;', "'")
    .replace(/\s+/g, ' ')
    .trim();

/** The sentence the answer card shows. */
const pageSentence = (html) =>
  (/<p class="sentence">([\s\S]*?)<\/p>/.exec(html)?.[1] ?? '')
    .replace(/<[^>]+>/g, '')
    .replaceAll('&#39;', "'")
    .replaceAll('&amp;', '&')
    .replace(/\s+/g, ' ')
    .trim();

// One server for the whole file: each host holds a worker thread.
let local;
const server = async () => (local ??= await localHandlers(root));
test.after(() => local?.host.close());

test('the checklist names at least twenty hero tools', () => {
  assert.ok(HERO.length >= 20, `only ${HERO.length} hero tools`);
});

test('every hero tool answers the same on both surfaces', async () => {
  const local = await server();
  const problems = [];
  let compared = { sentences: 0, answers: 0, citations: 0 };
  for (const id of HERO) {
    const tool = local.catalog.tools.find((t) => t.id === id);
    if (!tool) {
      problems.push(`${id}: on the checklist but not in the catalog`);
      continue;
    }
    // No args: the server runs the tool's own primary example, which is the
    // example the page ships its answer for.
    const result = await local.handlers.geoprims_run({ id });
    const html = page(id);
    if (!result.ok) {
      problems.push(`${id}: the server refused its own example`);
      continue;
    }
    // The sentence, word for word.
    const sentence = pageSentence(html);
    if (sentence) compared.sentences += 1;
    if (sentence && result.summary !== sentence) {
      problems.push(`${id}: the page says "${sentence}" and the server says "${result.summary}"`);
    }
    // The answer card's big number, which the page renders from `display`.
    const shown = pageAnswer(html);
    if (shown) {
      compared.answers += 1;
      const first = Object.keys(tool.outputs.properties)[0];
      const text = result.display?.[first];
      if (text && !shown.startsWith(String(text).split(' ')[0])) {
        problems.push(`${id}: the card shows "${shown}" and the server's ${first} is "${text}"`);
      }
    }
    // The citations, which now travel with every result.
    const refs = result.meta?.references ?? [];
    compared.citations += refs.length;
    if (refs.length !== tool.references.length) {
      problems.push(`${id}: ${refs.length} citations from the server, ${tool.references.length} in the manifest`);
    }
    for (const r of refs) {
      if (!html.includes(r.title.replaceAll('&', '&amp;').replaceAll("'", '&#39;'))) {
        problems.push(`${id}: the server cites "${r.title}", which the page does not show`);
      }
    }
  }
  assert.deepEqual(problems, []);
  // The sweep is only worth anything if it actually compared something.
  assert.ok(compared.sentences >= 20, `only ${compared.sentences} sentences compared`);
  assert.ok(compared.answers >= 20, `only ${compared.answers} answers compared`);
  assert.ok(compared.citations >= 40, `only ${compared.citations} citations compared`);
});

test('the sweep notices a surface saying something the other does not', () => {
  // The page's sentence and the server's summary are compared as text, so a
  // single changed word is caught.
  const html = '<p class="sentence">Density altitude is 7,932 ft.</p>';
  assert.equal(pageSentence(html), 'Density altitude is 7,932 ft.');
  assert.notEqual(pageSentence(html), 'Density altitude is 7,930 ft.');
  assert.equal(pageAnswer('<div class="value">7,932<span class="unit"> ft</span></div>'), '7,932 ft');
});

test('a hero tool that shows its work shows the same work to an agent', async () => {
  const local = await server();
  const problems = [];
  let checked = 0;
  for (const id of HERO) {
    const result = await local.handlers.geoprims_run({ id, explain: true });
    const trace = result.trace ?? [];
    if (!trace.length) continue;
    checked += 1;
    const html = page(id);
    for (const step of trace) {
      // The page prints each step's substituted formula under "Show your work".
      const printed = step.substituted.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll("'", '&#39;').replaceAll('"', '&quot;');
      if (!html.includes(printed)) problems.push(`${id}: the agent sees "${step.substituted}", which the page does not print`);
    }
  }
  assert.deepEqual(problems, []);
  assert.ok(checked >= 20, `only ${checked} hero tools showed their work`);
});
