// A forward tool and its inverse that give their primary example the same
// title must mean the same point. Run after the build: the examples run
// through the built Wasm modules.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { nodeHost } from '../../packages/runtime/src/node.mjs';
import { pairProblems, titledPairs } from './example-pairs.mjs';

const root = new URL('../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));

test('a forward and inverse pair naming one example mean one point', async () => {
  const host = nodeHost(join(root, 'dist/wasm'));
  assert.ok(titledPairs(catalog).length > 0, 'no pair shares an example title, so the gate checks nothing');
  assert.deepEqual(await pairProblems({ catalog, host }), []);
});

test('the gate catches an example moved off the point its pair names', async () => {
  const host = nodeHost(join(root, 'dist/wasm'));
  const pairs = titledPairs(catalog);
  const { inverse } = pairs[0];
  // Shift the inverse's example by a metre on its first length input, which is
  // how the Spire of Dublin came to be in two places at once.
  const moved = JSON.parse(JSON.stringify(catalog));
  const t = moved.tools.find((x) => x.id === inverse.id);
  const e = t.examples.find((x) => x.id === t['x-primary-example']) ?? t.examples[0];
  const key = Object.keys(e.input).find((k) => /^-?[\d.]+ ?\w*$/.test(String(e.input[k])));
  assert.ok(key, `the ${inverse.id} example has no plain numeric input to move`);
  const [, n, unit] = /^(-?[\d.]+) ?(\w*)$/.exec(String(e.input[key]));
  e.input[key] = `${Number(n) + 1}${unit ? ` ${unit}` : ''}`;
  const problems = await pairProblems({ catalog: moved, host });
  assert.ok(
    problems.some((p) => p.includes(inverse.id) && p.includes('is not one point')),
    `moving the example a metre was not caught: ${problems.join('; ') || 'no problems'}`,
  );
});
