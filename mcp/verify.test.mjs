import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { verifyRelease } from './verify.mjs';

test('release verification rejects a changed Wasm module before startup', () => {
  const root = mkdtempSync(join(tmpdir(), 'geoprims-verify-'));
  const wasm = join(root, 'dist/wasm');
  mkdirSync(wasm, { recursive: true });
  mkdirSync(join(root, 'dist/catalog'), { recursive: true });
  const bytes = Buffer.from('known module');
  writeFileSync(join(wasm, 'base.wasm'), bytes);
  writeFileSync(join(wasm, 'modules.json'), JSON.stringify({
    modules: [{ module: 'base', sha256: createHash('sha256').update(bytes).digest('hex') }],
  }));
  writeFileSync(join(root, 'dist/catalog/v1.json'), '{}');
  assert.equal(verifyRelease(root), 1);
  writeFileSync(join(wasm, 'base.wasm'), 'changed module');
  assert.throws(() => verifyRelease(root), /base\.wasm: expected .* got/);
});
