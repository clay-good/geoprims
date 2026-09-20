// Every status phrase the build can produce is one of the allowed shapes and
// claims nothing about safety, legality, or approval.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { bannedWord, statusFields, statusProblems } from './status.mjs';
import { nodeHost } from '../../packages/runtime/src/node.mjs';

const root = new URL('../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const host = nodeHost(join(root, 'dist/wasm'));
const withStatus = catalog.tools.filter((t) => statusFields(t).length > 0);

test('some tool declares a status, or this gate is checking nothing', () => {
  assert.ok(withStatus.length > 0);
});

test('every example of every status tool produces an allowed phrase', async () => {
  const problems = [];
  for (const t of withStatus) {
    for (const ex of t.examples) {
      const result = JSON.parse(await host.invoke(t.id, JSON.stringify(ex.input)));
      problems.push(...statusProblems(t, result));
    }
  }
  assert.deepEqual(problems, []);
});

test('the crosswind scenario: beyond the limit the reader entered', async () => {
  const t = catalog.tools.find((x) => x.id === 'aviation.wind.runway-components');
  const run = (args) => host.invoke(t.id, JSON.stringify(args)).then(JSON.parse);
  const base = { runway: '27', wind_direction: '300 deg', max_crosswind: '15 kt' };
  assert.equal((await run({ ...base, wind_speed: '40 kt' })).result.crosswind_status, 'Beyond your 15 kt crosswind limit');
  assert.equal((await run({ ...base, wind_speed: '29 kt' })).result.crosswind_status, 'Near your 15 kt crosswind limit');
  assert.equal((await run({ ...base, wind_speed: '10 kt' })).result.crosswind_status, 'Within your 15 kt crosswind limit');
  assert.equal((await run({ runway: '27', wind_direction: '300 deg', wind_speed: '40 kt' })).result.crosswind_status, undefined, 'no limit, no status');
});

test('a phrase that claims safety or legality fails, naming the word', () => {
  const tool = { id: 'fixture.s', outputs: { properties: { s: { 'x-status': { kind: 'threshold' } } } } };
  assert.deepEqual(statusProblems(tool, { result: { s: 'Within your safe limit' } }), [
    'fixture.s.s: a status phrase may not say "safe" ("Within your safe limit")',
  ]);
  assert.deepEqual(statusProblems(tool, { result: { s: 'Looks fine' } }), [
    'fixture.s.s: "Looks fine" does not start with one of "Within your", "Near your", "Beyond your"',
  ]);
  assert.equal(bannedWord('Within the safety margin'), undefined, 'safety is a different word');
});
