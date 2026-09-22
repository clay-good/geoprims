// feedback/problem-reports "No identifying fields": the report payload holds
// only the fields contracts/report-api lists, and the build fails if any other
// field is added. The key list is read from the contract itself, so a new field
// needs a spec change first; the Worker's accepted keys, the web dialog's
// payload, and (through the Worker's list, see mcp/server.test.mjs) the MCP
// report must all match it.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { KEYS, ROW_KEYS, DISPLAY_KEYS } from '../../worker/src/report.mjs';
import { buildPayload } from '../../apps/web/src/lib/report.js';

const spec = readFileSync(new URL('../../openspec/changes/define-build-contracts/specs/contracts/report-api/spec.md', import.meta.url), 'utf8');

/** The contract's "Submit endpoint" keys, in order, with the row and display keys. */
export function contractKeys(text) {
  const start = text.indexOf('### Requirement: Submit endpoint');
  const block = text.slice(start, text.indexOf('\n\n', text.indexOf('exactly these keys', start) + 20));
  const lines = block.split('\n').filter((l) => l.startsWith('- '));
  const top = lines.flatMap((l) => [...l.split(':')[0].matchAll(/`(\w+)`/g)].map((m) => m[1]));
  const nested = (name) => {
    const line = lines.find((l) => l.startsWith(`- \`${name}\``));
    return [...(/\{([^}]*)\}/.exec(line)?.[1] ?? '').matchAll(/\w+/g)].map((m) => m[0]);
  };
  return { top, row: nested('inputs'), display: nested('display') };
}

const contract = contractKeys(spec);

test('the contract lists the fields the spec names, and nothing identifying', () => {
  assert.deepEqual(contract.top, ['apiVersion', 'toolId', 'toolVersion', 'coreVersion', 'buildHash', 'assetVersions', 'kind', 'pagePath', 'inputs', 'outputs', 'warnings', 'display', 'note', 'token']);
  assert.deepEqual(contract.row, ['field', 'label', 'value', 'unit']);
  assert.deepEqual(contract.display, ['theme', 'unitProfile', 'viewportClass']);
  // Nothing the spec forbids: user agent, screen, language, time zone, referrer, address, identifier.
  for (const k of contract.top) assert.doesNotMatch(k, /agent|screen|lang|zone|referr|address|^ip|user|session|device/i);
});

test('the Worker accepts exactly the contract keys', () => {
  assert.deepEqual(KEYS, contract.top);
  assert.deepEqual(ROW_KEYS, contract.row);
  assert.deepEqual(DISPLAY_KEYS, contract.display);
});

const tool = {
  id: 'units.speed.convert',
  version: '1.0.0',
  coreVersion: '0.1.0',
  buildHash: 'abcdef0123456789',
  inputs: { properties: { value: { title: 'Value', 'x-unit': 'kt' }, to: { title: 'To' } } },
  outputs: { properties: { value: { title: 'Result' } } },
};
const result = { ok: true, result: { value: { value: 115.08, unit: 'mph' } }, meta: { warnings: [], assets: [] } };
const payload = (includeInputs) =>
  buildPayload({ tool, args: { value: 100, to: 'mph' }, result, includeInputs, note: 'x', kind: 'wrong-result', display: { theme: 'paper', unitProfile: 'default', viewportClass: 'phone' }, pagePath: '/units/speed/convert/#v1:abc', token: 't' });

test('the web dialog builds exactly the contract keys, with or without inputs', () => {
  for (const p of [payload(true), payload(false)]) {
    assert.deepEqual(Object.keys(p), contract.top);
    assert.deepEqual(Object.keys(p.display), contract.display);
    for (const r of [...p.inputs, ...p.outputs]) assert.deepEqual(Object.keys(r), contract.row);
  }
});

test('the lock fails when a field is added', () => {
  const extra = spec.replace('- `token`', '- `token`\n- `userAgent`');
  assert.notDeepEqual(contractKeys(extra).top, KEYS);
  assert.notDeepEqual(Object.keys({ ...payload(true), userAgent: 'x' }), contract.top);
});
