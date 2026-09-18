#!/usr/bin/env node
// Builds every core module to dist/wasm/<module>.wasm, then gates it:
// pinned toolchain, empty import section, Brotli budget. Writes
// dist/wasm/modules.json with sizes and SHA-256 digests.
//
// Usage: node tools/wasm/build.mjs [--skip-opt]
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { checkBudget, lintImports } from './lint.mjs';

const root = new URL('../..', import.meta.url).pathname;
const pins = JSON.parse(readFileSync(join(root, 'tools/toolchain.json'), 'utf8'));
const run = (cmd, args, opts = {}) => (execFileSync(cmd, args, { encoding: 'utf8', ...opts }) ?? '').trim();

// 1. Pinned versions (task 1.2). rustup honors rust-toolchain.toml from core/.
const rustc = run('rustc', ['--version'], { cwd: join(root, 'core') });
const wasmOpt = run('wasm-opt', ['--version']);
console.log(`rustc: ${rustc}\nwasm-opt: ${wasmOpt}`);
if (!rustc.startsWith(`rustc ${pins.rustc} `)) {
  throw new Error(`rustc ${pins.rustc} required (rust-toolchain.toml); got "${rustc}". Put rustup's cargo first on PATH.`);
}
const wasmLib = run('rustc', ['--print', 'target-libdir', '--target', 'wasm32-unknown-unknown'], { cwd: join(root, 'core') });
if (!existsSync(wasmLib)) {
  throw new Error(`rustc has no wasm32-unknown-unknown target (${wasmLib} missing). Use rustup's toolchain.`);
}
if (!new RegExp(`\\bversion ${pins.wasmOpt}\\b`).test(wasmOpt)) {
  throw new Error(`wasm-opt ${pins.wasmOpt} required; got "${wasmOpt}"`);
}

// 2. Build all module crates.
run('cargo', ['build', '--release', '--target', 'wasm32-unknown-unknown', '--workspace'], {
  cwd: join(root, 'core'),
  stdio: 'inherit',
});

// 3. Optimize, lint, budget, digest. wasm-opt may use only the features rustc's
// wasm32-unknown-unknown target enables by default (no SIMD, no relaxed SIMD).
const WASM_FEATURES = [
  '--enable-bulk-memory', '--enable-bulk-memory-opt', '--enable-call-indirect-overlong',
  '--enable-multivalue', '--enable-mutable-globals', '--enable-nontrapping-float-to-int',
  '--enable-reference-types', '--enable-sign-ext',
];
const out = join(root, 'dist/wasm');
mkdirSync(out, { recursive: true });
const modules = [];
for (const crate of readdirSync(join(root, 'core/crates')).sort()) {
  const toml = readFileSync(join(root, 'core/crates', crate, 'Cargo.toml'), 'utf8');
  const name = /\[package\.metadata\.geoprims\]\s*module\s*=\s*"([^"]+)"/.exec(toml)?.[1];
  if (!name) continue; // gp-base is a library, not a module
  const built = join(root, 'core/target/wasm32-unknown-unknown/release', `${crate.replaceAll('-', '_')}.wasm`);
  const dest = join(out, `${name}.wasm`);
  if (process.argv.includes('--skip-opt')) {
    writeFileSync(dest, readFileSync(built));
  } else {
    run('wasm-opt', ['-Oz', ...WASM_FEATURES, '--strip-debug', '--strip-producers', built, '-o', dest]);
  }
  const bytes = readFileSync(dest);
  lintImports(bytes, name);
  const budget = name === 'base' ? pins.budgets.base : pins.budgets.domain;
  const brotli = checkBudget(bytes, name, budget);
  const sha256 = createHash('sha256').update(bytes).digest('hex');
  modules.push({ module: name, crate, bytes: bytes.length, brotli, budget, sha256 });
}
writeFileSync(join(out, 'modules.json'), JSON.stringify({ rustc, wasmOpt, modules }, null, 2) + '\n');
console.table(modules.map(({ module, bytes, brotli, budget }) => ({ module, bytes, brotli, budget })));
