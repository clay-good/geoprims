// The report payload the dialog builds passes the Worker's own validator, holds
// only the contract's fields, and drops x-private inputs (feedback/problem-reports).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { buildPayload, reportText } from '../src/lib/report.js';
import { KEYS, validate } from '../../../worker/src/report.mjs';
import { nodeHost } from '../../../packages/runtime/src/node.mjs';

const root = new URL('../../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const versions = new Map(catalog.tools.map((t) => [t.id, t.version]));
const host = nodeHost(join(root, 'dist/wasm'));
const display = { theme: 'light', unitProfile: 'default', viewportClass: 'phone' };

async function payloadFor(t, over = {}) {
  const ex = (t.examples.find((e) => e.id === t['x-primary-example']) ?? t.examples[0]).input;
  const result = JSON.parse(await host.invoke(t.id, JSON.stringify(ex)));
  const tool = { ...t, coreVersion: catalog.coreVersion, buildHash: '0123456789abcdef' };
  return buildPayload({ tool, args: ex, result, includeInputs: true, note: 'Expected a different value', kind: 'wrong-result', display, pagePath: `/${t.id.split('.').join('/')}/#v1:x`, token: 't', ...over });
}

test('every tool: the dialog payload passes the Worker validator', async () => {
  for (const t of catalog.tools) {
    const p = await payloadFor(t);
    assert.deepEqual(Object.keys(p).sort(), [...KEYS].sort(), t.id);
    assert.ok(validate(p, versions), `${t.id} payload rejected by the Worker`);
  }
});

test('no identifying fields in the payload schema', async () => {
  const p = await payloadFor(catalog.tools[0]);
  const text = JSON.stringify(p);
  for (const banned of ['userAgent', 'screen', 'language', 'timeZone', 'referrer', 'ip']) assert.ok(!Object.hasOwn(p, banned), banned);
  assert.ok(!/Mozilla|Chrome|Safari/.test(text));
});

test('excluding inputs drops rows and the permalink fragment', async () => {
  const t = catalog.tools.find((x) => x.id === 'aviation.altimetry.density-altitude');
  const p = await payloadFor(t, { includeInputs: false });
  assert.deepEqual(p.inputs, []);
  assert.deepEqual(p.outputs, []);
  assert.equal(p.pagePath, '/aviation/altimetry/density-altitude/');
});

test('x-private inputs are never sent and outputs are withheld', async () => {
  const t = structuredClone(catalog.tools.find((x) => x.id === 'aviation.altimetry.density-altitude'));
  t.inputs.properties.elevation['x-private'] = true;
  const p = await payloadFor(t);
  assert.ok(!p.inputs.some((r) => r.field === 'elevation'));
  assert.ok(p.outputs.every((r) => r.value === '(withheld)'));
});

test('copy-report text carries no token', async () => {
  const p = await payloadFor(catalog.tools[0], { token: 'secret-token' });
  assert.ok(!reportText(p).includes('secret-token'));
});
