// Loads every built module (run `npm run build:wasm` first) and calls the ABI.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const root = new URL('../..', import.meta.url).pathname;
const { modules } = JSON.parse(readFileSync(join(root, 'dist/wasm/modules.json'), 'utf8'));

for (const { module, sha256 } of modules) {
  test(`${module}.wasm instantiates with no imports and reports its version`, async () => {
    const bytes = readFileSync(join(root, 'dist/wasm', `${module}.wasm`));
    const { instance } = await WebAssembly.instantiate(bytes, {});
    const { gp_version, gp_out_len, memory } = instance.exports;
    const ptr = gp_version();
    const text = new TextDecoder().decode(new Uint8Array(memory.buffer, ptr, gp_out_len()));
    assert.match(text, new RegExp(`^${module}@\\d+\\.\\d+\\.\\d+$`));
    assert.equal(sha256.length, 64);
  });
}
