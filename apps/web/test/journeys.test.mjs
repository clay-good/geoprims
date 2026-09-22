// Learning guides (web/tool-docs, "Learning guides"): the eight journeys of
// plan-launch-and-value-proof L3, each a chain that runs through the core, and
// each step's link opening its tool with the previous steps' outputs in it.
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { asInput } from '../src/lib/journeys.mjs';
import { JOURNEYS, journeyGuides } from '../src/lib/catalog.mjs';
import { nodeHost } from '../../../packages/runtime/src/node.mjs';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const host = nodeHost(join(root, 'dist/wasm'));
const link = await host.module('link');
const decode = async (href) => JSON.parse(await link.callString('gp_link_decode', href.split('#')[1]));

test('the eight launch journeys, and only those, are published', () => {
  const design = readFileSync(join(root, 'openspec/changes/plan-launch-and-value-proof/design.md'), 'utf8');
  const listed = [...design.slice(design.indexOf('### L3.')).split('\n### ')[0].matchAll(/^\d+\. (.+)$/gm)].map((m) => m[1].trim());
  assert.equal(listed.length, 8);
  assert.deepEqual(JOURNEYS.map((j) => j.title), listed, 'data/journeys.json follows the L3 list, in order');
});

test('guide chain works: each step opens with the previous steps’ outputs', async () => {
  const guides = await journeyGuides();
  for (const g of guides) {
    for (const [i, s] of g.steps.entries()) {
      const state = await decode(s.href);
      assert.ok(state.ok, `${g.slug} step ${i + 1}: the link does not decode`);
      assert.deepEqual(state.result.state.i, s.input, `${g.slug} step ${i + 1}: the link holds the step's inputs`);
      for (const c of s.carried) {
        const raw = JOURNEYS.find((j) => j.slug === g.slug).steps[i].input[c.name];
        const out = g.steps[c.from].result.result[raw.from[1]];
        if (!raw.columns) assert.deepEqual(s.input[c.name], asInput(out), `${g.slug} step ${i + 1}: ${c.name} is step ${c.from + 1}'s ${raw.from[1]}`);
      }
      if (s.carried.length) assert.equal(state.result.state.c, g.steps[s.carried[0].from].tool, 'names the step it came from');
    }
  }
  // The spec's scenario, spelled out: Drone mapping day carries the height forward.
  const drone = guides.find((g) => g.slug === 'drone-mapping-day');
  const height = drone.steps[0].result.result.height;
  assert.equal(drone.steps[3].input.height, asInput(height));
  assert.ok(Math.abs(parseFloat(drone.steps[3].input.height) - height.value) < 1e-9);
  assert.equal(drone.steps[5].input.line_spacing, asInput(drone.steps[3].result.result.line_spacing));
});

test('quantities travel as "value unit", lists and plain values as they are', () => {
  assert.equal(asInput({ value: 72.96000000000001, unit: 'm' }), '72.96 m');
  assert.equal(asInput({ value: 0.9997, unit: '1' }), 0.9997);
  assert.equal(asInput('892a8471487ffff'), '892a8471487ffff');
  assert.deepEqual(asInput([{ direction: 'N 1 E' }]), [{ direction: 'N 1 E' }]);
});

test('journey pages are built, linked from home and from the hubs of their tools', () => {
  const dist = join(web, 'dist');
  if (!existsSync(join(dist, 'index.html'))) return;
  const home = readFileSync(join(dist, 'index.html'), 'utf8');
  for (const j of JOURNEYS) {
    const page = readFileSync(join(dist, 'journeys', j.slug, 'index.html'), 'utf8');
    assert.equal((page.match(/class="card journey-step"/g) ?? []).length, j.steps.length, j.slug);
    assert.ok(home.includes(`href="/journeys/${j.slug}/"`), `${j.slug}: not on the home page`);
  }
  assert.match(readFileSync(join(dist, 'drone/photogrammetry/index.html'), 'utf8'), /href="\/journeys\/drone-mapping-day\/"/);
});
