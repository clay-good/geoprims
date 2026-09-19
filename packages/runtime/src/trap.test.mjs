// compute-core "Trap is contained" and MCP "Resource limits and robustness":
// a Wasm trap becomes an INTERNAL envelope, the instance is replaced, and the
// next call works. The fixture is test-fixtures/trap.wat.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { loadModule } from './module.mjs';

const bytes = readFileSync(new URL('../test-fixtures/trap.wasm', import.meta.url));

test('a trap is contained, reported as INTERNAL, and the instance is replaced', async () => {
  const m = await loadModule(bytes, 'fixture');
  assert.deepEqual(JSON.parse(await m.invoke('ok.a.b', '{}')), { ok: true, calls: 1 });
  assert.deepEqual(JSON.parse(await m.invoke('ok.a.b', '{}')), { ok: true, calls: 2 });
  const trap = JSON.parse(await m.invoke('trap.a.b', '{}'));
  assert.equal(trap.ok, false);
  assert.equal(trap.error.code, 'INTERNAL');
  assert.match(trap.error.message, /fixture module stopped while running trap\.a\.b \(trap\)/);
  // A fresh instance: its call count starts again.
  assert.deepEqual(JSON.parse(await m.invoke('ok.a.b', '{}')), { ok: true, calls: 1 });
});
