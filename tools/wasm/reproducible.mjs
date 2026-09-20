#!/usr/bin/env node
// Reproducible-build check (platform/verification, "Reproducible builds").
// Builds the Wasm modules twice from a clean target directory and compares the
// digests. A geoprims result is only checkable by someone else if the same
// source gives them the same bytes.
//
//   node tools/wasm/reproducible.mjs
//
// It is slow (two full release builds), so it is a local or release-time check.
// CI separately compares every shipped artifact from two independent runners.
import { execFileSync } from 'node:child_process';
import { readFileSync, rmSync } from 'node:fs';
import { join } from 'node:path';

const root = new URL('../..', import.meta.url).pathname;
const build = join(root, 'tools/wasm/build.mjs');

/** Builds once from a clean target directory and returns {module: sha256}. */
function digests(label) {
  rmSync(join(root, 'core/target/wasm32-unknown-unknown'), { recursive: true, force: true });
  process.stderr.write(`${label}: building…\n`);
  execFileSync(process.execPath, [build], { cwd: root, stdio: ['ignore', 'ignore', 'inherit'] });
  const { modules } = JSON.parse(readFileSync(join(root, 'dist/wasm/modules.json'), 'utf8'));
  return Object.fromEntries(modules.map((m) => [m.module, m.sha256]));
}

const first = digests('build 1');
const second = digests('build 2');
const problems = Object.keys(first)
  .filter((m) => first[m] !== second[m])
  .map((m) => `${m}.wasm: ${first[m]} then ${second[m]}`);

if (problems.length > 0) {
  console.error(`not reproducible:\n${problems.map((p) => `  ${p}`).join('\n')}`);
  process.exit(1);
}
console.log(`reproducible: ${Object.keys(first).length} modules, identical digests across two clean builds`);
for (const [m, sha] of Object.entries(first)) console.log(`  ${sha}  ${m}.wasm`);
