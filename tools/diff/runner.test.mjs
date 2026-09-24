// The differential runner, in brief: each family with its reference
// installed agrees on 300 cases, and a deliberately perturbed tool fails.
// The full run (10,000 cases per family) is `npm run diff`.
import { spawnSync } from 'node:child_process';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { nodeHost } from '../../packages/runtime/src/node.mjs';
import { FAMILIES, runFamily } from './runner.mjs';

const root = new URL('../..', import.meta.url).pathname;
const host = nodeHost(join(root, 'dist/wasm'));
/** Installed if it starts at all: PROJ's `proj` exits 1 on --version. */
const installed = (cmd) => !spawnSync(cmd, ['--version'], { stdio: 'ignore' }).error;

for (const fam of FAMILIES) {
  test(`${fam.name}: ${fam.tool} agrees with ${fam.needs}`, { skip: !installed(fam.needs) && `${fam.needs} is not installed` }, async () => {
    const out = await runFamily(host, fam, 300);
    assert.deepEqual(out.failures.slice(0, 5), [], `${out.failures.length} of ${out.cases} differ`);
  });
}

test('a deliberately perturbed tool fails the comparison', { skip: !installed('GeodSolve') && 'GeodSolve is not installed' }, async () => {
  const fam = FAMILIES.find((f) => f.name === 'geodesic-inverse');
  // Ten times the declared tolerance (1e-9 of the distance), plus 1 µm so
  // zero-length lines are perturbed too: still far inside any everyday need.
  const perturb = (r) => ({ ...r, distance: { ...r.distance, value: r.distance.value * (1 + 1e-8) + 1e-6 } });
  const out = await runFamily(host, fam, 300, 20260922, perturb);
  assert.ok(out.failures.length >= 290, `only ${out.failures.length} of 300 caught`);
  assert.match(out.failures[0].problem, /^distance/);
});
