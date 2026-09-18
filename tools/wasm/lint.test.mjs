// Proves the gates fail on deliberately bad fixture crates (tasks 1.4 and 1.5).
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { checkBudget, lintImports, listImports } from './lint.mjs';

const root = new URL('../..', import.meta.url).pathname;

function buildFixture(name) {
  const pin = JSON.parse(readFileSync(join(root, 'tools/toolchain.json'), 'utf8')).rustc;
  const rustc = execFileSync('rustc', ['--version'], { cwd: join(root, 'core'), encoding: 'utf8' });
  assert.ok(rustc.startsWith(`rustc ${pin} `), `needs rustup's rustc ${pin} first on PATH; got ${rustc}`);
  const dir = join(root, 'core/fixtures', name);
  execFileSync('cargo', ['build', '-q', '--release', '--target', 'wasm32-unknown-unknown'], { cwd: dir, stdio: 'inherit' });
  return readFileSync(join(dir, 'target/wasm32-unknown-unknown/release', `fixture_${name.replaceAll('-', '_')}.wasm`));
}

test('import lint rejects a module that imports JS Math', () => {
  const bytes = buildFixture('bad-imports');
  assert.deepEqual(listImports(bytes), [{ module: 'Math', name: 'sin' }]);
  assert.throws(() => lintImports(bytes, 'bad-imports'), /forbidden Wasm imports \(Math\.sin\)/);
});

test('size gate rejects an oversized module, naming size and budget', () => {
  const bytes = buildFixture('oversized');
  assert.throws(() => checkBudget(bytes, 'oversized', 409600), /oversized: \d+ bytes Brotli exceeds its budget of 409600 bytes/);
});

test('size gate passes a small module', () => {
  const bytes = new Uint8Array(1000);
  assert.ok(checkBudget(bytes, 'tiny', 122880) < 100);
});
