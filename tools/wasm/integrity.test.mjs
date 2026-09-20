// Build integrity (platform/verification, "Reproducible builds"). The digests
// the build recorded must match the modules on disk, and the MCP release must
// ship those exact bytes. The website's half of the same check lives in
// apps/web/test/build.test.mjs, because public/wasm is only prepared once the
// site is built.
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const root = new URL('../..', import.meta.url).pathname;
const wasmDir = join(root, 'dist/wasm');
const { modules } = JSON.parse(readFileSync(join(wasmDir, 'modules.json'), 'utf8'));
const sha = (path) => createHash('sha256').update(readFileSync(path)).digest('hex');

test('every recorded digest matches the module on disk', () => {
  const problems = [];
  for (const m of modules) {
    const file = join(wasmDir, `${m.module}.wasm`);
    if (!existsSync(file)) {
      problems.push(`${m.module}.wasm is recorded but not built`);
      continue;
    }
    const actual = sha(file);
    if (actual !== m.sha256) problems.push(`${m.module}.wasm is ${actual}, recorded as ${m.sha256}`);
    if (readFileSync(file).length !== m.bytes) problems.push(`${m.module}.wasm is not ${m.bytes} bytes`);
  }
  assert.deepEqual(problems, []);
});

test('every built module is recorded, and nothing else is shipped', () => {
  const built = readdirSync(wasmDir).filter((f) => f.endsWith('.wasm')).sort();
  const recorded = modules.map((m) => `${m.module}.wasm`).sort();
  assert.deepEqual(built, recorded);
});

test('the MCP release ships byte-identical modules', { skip: existsSync(join(root, 'mcp/dist/wasm')) ? false : 'mcp/dist is not built' }, () => {
  const there = join(root, 'mcp/dist/wasm');
  const problems = [];
  for (const m of modules) {
    const file = join(there, `${m.module}.wasm`);
    if (!existsSync(file)) problems.push(`the MCP release is missing ${m.module}.wasm`);
    else if (sha(file) !== m.sha256) problems.push(`the MCP release's ${m.module}.wasm is not the built one`);
  }
  assert.deepEqual(problems, []);
});

test('a tampered module is caught', () => {
  const m = modules[0];
  const bytes = readFileSync(join(wasmDir, `${m.module}.wasm`));
  const tampered = Buffer.from(bytes);
  tampered[tampered.length - 1] ^= 0xff;
  assert.notEqual(createHash('sha256').update(tampered).digest('hex'), m.sha256);
});
