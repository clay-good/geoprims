// Workflows (add-job-workflows): each job runs through the core by the shared
// chain runner, carries earlier answers forward, lists what it fixes, stops at
// a failed step, and is published at /workflows/<slug>/ with the old journey
// links redirecting to it.
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { asInput, assumptions, examples, runChain } from '../../../packages/runtime/src/chain.mjs';
import { catalog, JOURNEYS, WORKFLOWS, workflowGuides } from '../src/lib/catalog.mjs';
import { nodeHost } from '../../../packages/runtime/src/node.mjs';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const host = nodeHost(join(root, 'dist/wasm'));
const link = await host.module('link');
const invoke = async (id, input) => JSON.parse(await host.invoke(id, JSON.stringify(input)));
const decode = async (href) => JSON.parse(await link.callString('gp_link_decode', href.split('#')[1]));
const tool = (id) => catalog.tools.find((t) => t.id === id);

test('the eight launch journeys are workflows, in the L3 order', () => {
  const design = readFileSync(join(root, 'openspec/changes/plan-launch-and-value-proof/design.md'), 'utf8');
  const listed = [...design.slice(design.indexOf('### L3.')).split('\n### ')[0].matchAll(/^\d+\. (.+)$/gm)].map((m) => m[1].trim());
  assert.equal(listed.length, 8);
  assert.deepEqual(JOURNEYS.map((j) => j.title), listed, 'data/workflows.json keeps the L3 journeys, in order');
});

test('every workflow input names a real tool field, and at most eight show before More options', () => {
  let reached = 0;
  for (const w of WORKFLOWS) {
    const shown = w.inputs.filter((i) => !i.advanced);
    assert.ok(shown.length >= 1 && shown.length <= 8, `${w.slug}: ${shown.length} visible inputs`);
    for (const i of w.inputs) {
      assert.ok(tool(i.tool)?.inputs.properties[i.field], `${w.slug}: ${i.name} names ${i.tool}.${i.field}, which does not exist`);
      const used = w.steps.some((s) => Object.values(s.input ?? {}).some((v) => v?.input === i.name));
      assert.ok(used, `${w.slug}: input ${i.name} feeds no step`);
      reached++;
    }
    for (const s of w.steps) for (const name of Object.keys(s.input ?? {})) assert.ok(tool(s.tool).inputs.properties[name], `${w.slug}: ${s.tool} takes no input "${name}"`);
    assert.ok(Number.isInteger(w.visual?.step) && w.visual.step < w.steps.length, `${w.slug}: visual names no step`);
  }
  assert.ok(reached > 40, `${reached} inputs checked`);
});

test('each step opens its tool with exactly the inputs it ran with, earlier answers carried in', async () => {
  const guides = await workflowGuides();
  assert.equal(guides.length, WORKFLOWS.length);
  for (const g of guides) {
    for (const [i, s] of g.steps.entries()) {
      assert.equal(s.status, 'ok', `${g.slug} step ${i + 1}`);
      const state = await decode(s.href);
      assert.ok(state.ok, `${g.slug} step ${i + 1}: the link does not decode`);
      assert.deepEqual(state.result.state.i, s.input, `${g.slug} step ${i + 1}: the link holds the step's inputs`);
      if (s.carried.length) assert.equal(state.result.state.c, g.steps[s.carried[0].from].tool, 'names the step it came from');
    }
  }
  // The spec's scenario: the mapping flight carries the height forward.
  const drone = guides.find((g) => g.slug === 'mapping-flight');
  const height = drone.steps[0].result.result.height;
  assert.equal(drone.steps[3].input.height, asInput(height));
  assert.equal(drone.steps[5].input.line_spacing, asInput(drone.steps[3].result.result.line_spacing));
});

test('a changed input changes every step that depends on it', async () => {
  const w = WORKFLOWS.find((x) => x.slug === 'mapping-flight');
  const a = await runChain(w, {}, invoke);
  const b = await runChain(w, { target_gsd: '3 cm' }, invoke);
  assert.ok(a.ok && b.ok);
  for (const i of [0, 2, 3, 4, 5, 6, 7]) assert.notDeepEqual(b.steps[i].result.result, a.steps[i].result.result, `step ${i + 1} did not follow the GSD`);
  assert.deepEqual(b.steps[8].result.result, a.steps[8].result.result, 'the sun window does not depend on the GSD');
});

test('a failed step stops the chain: its own error, and later steps wait', async () => {
  const w = WORKFLOWS.find((x) => x.slug === 'preflight-check');
  const r = await runChain(w, { metar: 'not a metar' }, invoke);
  assert.equal(r.ok, false);
  assert.equal(r.failed, 0);
  assert.equal(r.steps[0].status, 'failed');
  assert.ok(r.steps[0].result.error.message.length > 10);
  for (const s of r.steps.slice(1)) {
    assert.equal(s.status, 'waiting');
    assert.equal(s.result, null, 'nothing ran on a guessed value');
  }
});

test('wiring mistakes are the workflow’s bug and throw, naming the step', async () => {
  const bad = { slug: 'x', inputs: [], steps: [{ tool: 'units.length.convert', input: { value: { from: [0, 'nope'] } } }] };
  await assert.rejects(runChain(bad, {}, invoke), /step 1: "value" takes from step 1, which is not earlier/);
  const missing = { slug: 'y', inputs: [], steps: [{ tool: 'geodesy.parse.coordinates', input: { text: '40 N 80 W' } }, { tool: 'geodesy.utm.forward', input: { lat: { from: [0, 'latitude'] } } }] };
  await assert.rejects(runChain(missing, {}, invoke), /has no output "latitude"/);
});

test('assumptions are the literals a workflow fixes, and the examples are its prefill', () => {
  const w = WORKFLOWS.find((x) => x.slug === 'preflight-check');
  assert.deepEqual(assumptions(w).map((a) => `${a.step}:${a.name}`), ['3:wind_reference']);
  assert.equal(examples(w).runway, '25');
});

test('quantities travel as "value unit", lists and plain values as they are', () => {
  assert.equal(asInput({ value: 72.96000000000001, unit: 'm' }), '72.96 m');
  assert.equal(asInput({ value: 0.9997, unit: '1' }), 0.9997);
  assert.equal(asInput('892a8471487ffff'), '892a8471487ffff');
  assert.deepEqual(asInput([{ direction: 'N 1 E' }]), [{ direction: 'N 1 E' }]);
});

test('journey links redirect to their workflows', () => {
  const redirects = JSON.parse(readFileSync(join(root, 'data/redirects.json'), 'utf8'));
  for (const w of JOURNEYS) {
    assert.ok(redirects.some((r) => r.from === `/journeys/${w.journey}/` && r.to === `/workflows/${w.slug}/`), `${w.journey} does not redirect`);
  }
});

test('workflow pages are built with their answers, and linked from home and the hubs', () => {
  const dist = join(web, 'dist');
  if (!existsSync(join(dist, 'index.html'))) return;
  const home = readFileSync(join(dist, 'index.html'), 'utf8');
  for (const w of WORKFLOWS) {
    const page = readFileSync(join(dist, 'workflows', w.slug, 'index.html'), 'utf8');
    assert.equal((page.match(/class="journey-step/g) ?? []).length, w.steps.length, w.slug);
    assert.ok(home.includes(`href="/workflows/${w.slug}/"`), `${w.slug}: not on the home page`);
  }
  // The answer is in the HTML before any script: the preflight's pressure altitude sentence.
  assert.match(readFileSync(join(dist, 'workflows/preflight-check/index.html'), 'utf8'), /pressure altitude/i);
  assert.ok(existsSync(join(dist, 'workflows/index.html')));
  assert.match(readFileSync(join(dist, 'drone/photogrammetry/index.html'), 'utf8'), /href="\/workflows\/mapping-flight\/"/);
  assert.ok(!existsSync(join(dist, 'journeys')), 'journey pages are redirects now');
});

test('the copied plan is the inputs and each step’s own sentence, with the link back', async () => {
  const { planText } = await import('../src/lib/plan.mjs');
  const w = WORKFLOWS.find((x) => x.slug === 'preflight-check');
  const r = await runChain(w, {}, invoke);
  const text = planText({ title: w.title, inputs: [{ title: 'Runway', value: '25' }], steps: r.steps.map((s) => ({ ...s, title: tool(s.tool).title })), url: 'https://geoprims.com/workflows/preflight-check/', today: '2026-09-25' });
  assert.match(text, /^VFR preflight \(geoprims, 2026-09-25\)\n/);
  for (const s of r.steps) assert.ok(text.includes(s.result.summary), 'a step’s summary is copied word for word');
  assert.match(text, /Not for primary navigation/);
  const stopped = await runChain(w, { metar: 'nope' }, invoke);
  assert.match(planText({ title: w.title, inputs: [], steps: stopped.steps.map((s) => ({ ...s, title: tool(s.tool).title })), url: 'u', today: 'd' }), /2\. Pressure altitude: Waiting for step 1\./);
});

test('a carry can reach into a list, count from its end, sit inside a literal, or be optional', async () => {
  const seen = [];
  const fake = async (tool, input) => {
    seen.push(input);
    return { ok: true, result: { rows: [{ v: { value: 1, unit: 'm' } }, { v: { value: 2, unit: 'm' } }], total: { value: 3, unit: 'h' } } };
  };
  const w = {
    slug: 'z',
    inputs: [{ name: 'burn', example: '9 gal/h' }],
    steps: [
      { tool: 'a', input: {} },
      { tool: 'b', input: { first: { from: [0, 'rows', 0, 'v'] }, last: { from: [0, 'rows', -1, 'v'] }, gust: { from: [0, 'gust'], optional: true }, legs: [{ time: { from: [0, 'total'] }, burn: { input: 'burn' } }], fixed: 'x' } },
    ],
  };
  const r = await runChain(w, {}, fake);
  assert.ok(r.ok);
  assert.deepEqual(seen[1], { first: '1 m', last: '2 m', legs: [{ time: '3 h', burn: '9 gal/h' }], fixed: 'x' });
  assert.deepEqual(r.steps[1].carried.map((c) => c.name), ['first', 'last', 'legs']);
  // Only the whole literal counts as an assumption; a literal holding a reference does not.
  assert.deepEqual(assumptions(w).map((a) => a.name), ['fixed']);
  // Without "optional", a missing answer is the workflow's bug.
  await assert.rejects(runChain({ ...w, steps: [w.steps[0], { tool: 'b', input: { gust: { from: [0, 'gust'] } } }] }, {}, fake), /has no output "gust"/);
});
