// The reproduction script on seeded reports: a faithful report reproduces
// with no differences, a report whose output was wrong is marked on exactly
// that field, and a report sent without inputs says it cannot reproduce.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { nodeHost } from '../../packages/runtime/src/node.mjs';
import { reproduce, rowsOf } from '../scripts/reproduce.mjs';

const root = new URL('../..', import.meta.url).pathname;
const wasm = join(root, 'dist/wasm');
const skip = existsSync(join(wasm, 'link.wasm')) ? false : 'needs npm run build';

async function seeded(host, toolId, inputs) {
  const link = await host.module('link');
  const enc = JSON.parse(await link.callString('gp_link_encode', JSON.stringify({ state: { i: inputs }, flags: ['report'] })));
  const out = JSON.parse(await host.invoke(toolId, JSON.stringify(inputs)));
  return {
    id: 'r1',
    tool_id: toolId,
    tool_version: '1.0.0',
    build_hash: 'x',
    kind: 'wrong-result',
    page_path: `/${toolId.split('.').join('/')}/#${enc.result.fragment}`,
    outputs_json: JSON.stringify(rowsOf(out).map((r) => ({ ...r, label: r.field }))),
  };
}

test('a faithful report reproduces on both builds with no differences', { skip }, async () => {
  const host = nodeHost(wasm);
  const row = await seeded(host, 'aviation.altimetry.pressure-altitude', { elevation: '5000 ft', altimeter: '29.80 inHg' });
  const r = await reproduce(row, { current: host, reported: host });
  assert.equal(r.reproducible, true);
  assert.deepEqual(r.args, { elevation: '5000 ft', altimeter: '29.80 inHg' });
  assert.deepEqual([r.saidVsThen, r.thenVsNow, r.saidVsNow], [[], [], []]);
  assert.ok(r.fields.includes('pressure_altitude'));
});

test('a wrong reported value is marked on that field only', { skip }, async () => {
  const host = nodeHost(wasm);
  const row = await seeded(host, 'aviation.altimetry.pressure-altitude', { elevation: '5000 ft', altimeter: '29.80 inHg' });
  const rows = JSON.parse(row.outputs_json);
  rows.find((x) => x.field === 'pressure_altitude').value = '5120';
  const r = await reproduce({ ...row, outputs_json: JSON.stringify(rows) }, { current: host });
  assert.deepEqual(r.saidVsNow, ['pressure_altitude']);
  assert.equal(r.then, null);
  assert.equal(r.thenVsNow, null);
});

test('a report without inputs says it cannot be reproduced', { skip }, async () => {
  const r = await reproduce({ page_path: '/aviation/altimetry/pressure-altitude/', outputs_json: '[]' }, { current: nodeHost(wasm) });
  assert.equal(r.reproducible, false);
  assert.match(r.why, /without its inputs/);
});
