// Every refusal a golden vector pins is one the tool's manifest declares.
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { undeclared } from './error-codes.mjs';

const root = new URL('../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));

test('every error code a vector expects is declared by its tool', () => {
  const problems = new Set();
  let refusals = 0;
  for (const t of catalog.tools) {
    const file = join(root, 'core/vectors', `${t.id}.jsonl`);
    if (!existsSync(file)) continue;
    for (const line of readFileSync(file, 'utf8').split('\n').filter(Boolean)) {
      const code = JSON.parse(line).expect['error.code'];
      if (!code) continue;
      refusals++;
      const why = undeclared(t, code);
      if (why) problems.add(why);
    }
  }
  assert.deepEqual([...problems], []);
  assert.ok(refusals > 200, `${refusals} refusals checked`);
});

test('a code missing from errors is caught, and the implicit ones are not', () => {
  const t = { id: 'fixture.tool', errors: ['OUT_OF_DOMAIN'] };
  assert.equal(undeclared(t, 'OUT_OF_DOMAIN'), null);
  assert.equal(undeclared(t, 'INVALID_INPUT'), null);
  assert.equal(undeclared(t, 'UNIT_MISMATCH'), null);
  assert.match(undeclared(t, 'NO_SOLUTION'), /fixture.tool returns NO_SOLUTION/);
});

test('a tool that takes a quantity publishes OUT_OF_DOMAIN, the runtime refusal of an absurd magnitude', () => {
  // A quantity takes a number or a unit-tagged string; a plain number field is not one.
  const quantity = (props) => Object.values(props).some((p) => (p['x-quantity'] && Array.isArray(p.type)) || (p.type === 'array' && quantity(p.items?.properties ?? {})));
  const missing = catalog.tools.filter((t) => quantity(t.inputs.properties) && !t.errors.includes('OUT_OF_DOMAIN')).map((t) => t.id);
  assert.deepEqual(missing, []);
});
