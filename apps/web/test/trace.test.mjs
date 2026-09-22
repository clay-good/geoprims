// "Show your work" (trust/proof-display). A tool that shows its work renders
// the same steps on the page that an agent gets from `explain: true`, and
// explaining never changes a number.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { nodeHost } from '../../../packages/runtime/src/node.mjs';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const dist = join(web, 'dist');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const host = nodeHost(join(root, 'dist/wasm'));
const DA = 'aviation.altimetry.density-altitude';
const primary = (t) => t.examples.find((e) => e.id === t['x-primary-example']) ?? t.examples[0];

/**
 * A trace ends at the answer its card leads with. These few tools conclude
 * somewhere else, and each one says why: the card leads with the number a
 * reader checks first, which is not always the last thing the arithmetic
 * produces. Everything absent from this list is held to the card's headline.
 */
const ENDS_ELSEWHERE = {
  // The card leads with total weight, the number against the aircraft's limit,
  // but the work ends at the centre of gravity it is used to find.
  'aviation.loading.weight-balance': 'cg',
  // The card leads with the radius, which the two given elements fix; the work
  // then derives the elements that follow from it, ending at the curve length.
  'survey.curves.circular-curve': 'length',
  // The card leads with the canonical tile id, which is the input written
  // back; the work ends at the centre the bounds are really asked for.
  'indexing.tile.bounds': 'lat',
  // The card leads with the zone, which names the grid; the work ends at
  // the easting, which is the position in it.
  'geodesy.utm.forward': 'easting',
};
const run = async (id, input) => JSON.parse(await host.invoke(id, JSON.stringify(input)));

test('explaining changes no number, on any tool', async () => {
  const problems = [];
  for (const t of catalog.tools) {
    const input = primary(t).input;
    const plain = await run(t.id, input);
    const shown = await run(t.id, { ...input, options: { ...(input.options ?? {}), explain: true } });
    if (JSON.stringify(plain.result) !== JSON.stringify(shown.result)) problems.push(`${t.id}: explaining moved a result`);
    if (plain.trace !== undefined) problems.push(`${t.id}: an ordinary call carries a trace`);
  }
  assert.deepEqual(problems, []);
});

test('every tool that shows its work ends at the answer the card shows', async () => {
  const problems = [];
  let showing = 0;
  for (const t of catalog.tools) {
    const input = primary(t).input;
    const shown = await run(t.id, { ...input, options: { ...(input.options ?? {}), explain: true } });
    if (!shown.trace) continue;
    showing += 1;
    if (shown.trace.length < 2) problems.push(`${t.id}: a trace of one step explains nothing`);
    const first = Object.keys(t.outputs.properties).find((k) => shown.display?.[k] !== undefined);
    const ends = ENDS_ELSEWHERE[t.id] ?? first;
    if (shown.trace.at(-1).value !== shown.display[ends]) {
      problems.push(`${t.id}: the last step reads "${shown.trace.at(-1).value}", the answer is "${shown.display[ends]}"`);
    }
    for (const step of shown.trace) {
      for (const [field, text] of Object.entries(step)) {
        if (!text) problems.push(`${t.id}: a step has an empty ${field}`);
      }
    }
  }
  assert.deepEqual(problems, []);
  assert.ok(showing >= 5, `only ${showing} tools show their work`);
});

test('the page shows the same steps, in the same words', async () => {
  const html = readFileSync(join(dist, 'aviation/altimetry/density-altitude/index.html'), 'utf8');
  const block = /<ol class="trace">([\s\S]*?)<\/ol>/.exec(html);
  assert.ok(block, 'the density-altitude page shows its work');
  const t = catalog.tools.find((x) => x.id === DA);
  const shown = await run(DA, { ...primary(t).input, options: { explain: true } });
  const text = block[1].replace(/<[^>]+>/g, ' ').replace(/&#39;/g, "'").replace(/&amp;/g, '&').replace(/\s+/g, ' ');
  for (const step of shown.trace) {
    for (const part of [step.label, step.value]) {
      assert.ok(text.includes(part), `the page is missing "${part}"`);
    }
  }
  // The intermediates the scenario names: pressure altitude and density.
  assert.match(text, /Pressure altitude/);
  assert.match(text, /Air density/);
  assert.match(text, /ρ = p \/ \(R × Tv\)/, 'the formula in standard notation');
});

test('a bad explain option is refused, not ignored', async () => {
  const t = catalog.tools.find((x) => x.id === DA);
  const bad = await run(DA, { ...primary(t).input, options: { explain: 'yes' } });
  assert.equal(bad.ok, false);
  assert.equal(bad.error.field, '/options/explain');
});

test('a decoder shows every coded group beside what it says', () => {
  const html = readFileSync(join(dist, 'aviation/weather/metar-decode/index.html'), 'utf8');
  const block = /<dl class="groups(?: no-print)?">([\s\S]*?)<\/dl>/.exec(html);
  assert.ok(block, 'the METAR decoder shows no decoded groups');
  const text = block[1].replace(/<[^>]+>/g, ' ').replace(/&#39;/g, "'").replace(/\s+/g, ' ');
  // Every group of the example report, and what each one says.
  for (const [group, meaning] of [
    ['30015G25KT', 'wind 300° true at 15 kt, gusting 25 kt'],
    ['10SM', 'visibility 10 statute miles'],
    ['A2980', 'altimeter 29.8 inHg'],
    ['30/08', 'temperature 30 °C, dew point 8 °C'],
  ]) {
    assert.ok(text.includes(group), `the block does not show ${group}`);
    assert.ok(text.includes(meaning), `${group} is not explained: expected "${meaning}"`);
  }
  // Every token of the report is accounted for, none quietly dropped.
  const report = 'KDEN 181753Z 30015G25KT 10SM FEW080 SCT200 30/08 A2980 RMK AO2 SLP052 T03000083';
  for (const token of report.split(' ')) assert.ok(text.includes(token), `${token} was not decoded`);
});

test('a tool with no decoder shows no groups block', () => {
  const html = readFileSync(join(dist, 'aviation/altimetry/density-altitude/index.html'), 'utf8');
  assert.doesNotMatch(html, /<dl class="groups(?: no-print)?">/);
});
