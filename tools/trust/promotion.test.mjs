// Promotion and dimension gates (trust/correctness-program). Run after the
// build: every stable tool must pass every layer, and the gates themselves
// must catch a missing derivation, a self-computed example, and a unit mismatch.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { nodeHost } from '../../packages/runtime/src/node.mjs';
import { lintDimensions } from './dimensions.mjs';
import { derivationProblems, promotionProblems } from './promotion.mjs';

const root = new URL('../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));

test('every stable tool meets the stable bar', async () => {
  const host = nodeHost(join(root, 'dist/wasm'));
  await (await host.module('search')).callString('gp_search_load', JSON.stringify(catalog.tools));
  const stable = catalog.tools.filter((t) => t.stability === 'stable');
  assert.ok(stable.length > 0, 'no stable tools');
  const failures = [];
  for (const tool of stable) {
    for (const p of await promotionProblems({ root, tool, host })) failures.push(`${tool.id}: ${p}`);
  }
  assert.deepEqual(failures, []);
});

test('a missing derivation blocks promotion, naming the layer', () => {
  const empty = mkdtempSync(join(tmpdir(), 'gp-promo-'));
  assert.deepEqual(derivationProblems(empty, 'time.sun.position'), ['(A) needs a derivation note at docs/derivations/time.sun.position.md']);
});

test('a self-computed worked example blocks promotion', () => {
  const dir = mkdtempSync(join(tmpdir(), 'gp-promo-'));
  mkdirSync(join(dir, 'docs/derivations'), { recursive: true });
  const note = readFileSync(join(root, 'docs/derivations/time.sun.position.md'), 'utf8').replace('- independent: yes', '- independent: no');
  writeFileSync(join(dir, 'docs/derivations/time.sun.position.md'), note);
  const problems = derivationProblems(dir, 'time.sun.position');
  assert.ok(problems.includes('(B) needs an independent worked example'), problems.join('\n'));
});

test('dimension lint: the catalog is clean and a unit mismatch is caught', () => {
  assert.deepEqual(lintDimensions(catalog), []);
  const field = (title, quantity, unit) => ({ type: 'number', title, 'x-quantity': quantity, 'x-unit': unit });
  const bad = {
    tools: [
      {
        id: 'fixture.bad',
        inputs: { properties: { height_ft: field('Height', 'length', 'm') } },
        outputs: { properties: { field: field('Field (nT)', 'dimensionless', '1'), bare: { type: 'number', title: 'Bare' } } },
      },
    ],
  };
  assert.deepEqual(lintDimensions(bad), [
    'fixture.bad input height_ft: the name ends in _ft, but the unit is m',
    'fixture.bad output field: the title says nT, but the field is dimensionless',
    'fixture.bad output bare: numeric field has no x-quantity and x-unit',
  ]);
});
