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

test('the steps end at the answer the card shows', async () => {
  const t = catalog.tools.find((x) => x.id === DA);
  const input = primary(t).input;
  const shown = await run(DA, { ...input, options: { explain: true } });
  assert.ok(shown.trace.length >= 3);
  const first = Object.keys(t.outputs.properties).find((k) => shown.display?.[k] !== undefined);
  assert.equal(shown.trace.at(-1).value, shown.display[first], 'the last step is the headline answer');
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
