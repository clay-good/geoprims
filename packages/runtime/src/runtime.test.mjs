// Runtime tests against the built modules (run `node tools/wasm/build.mjs` first).
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { checkJson } from './harden.mjs';
import { nodeHost } from './node.mjs';

const root = new URL('../../..', import.meta.url).pathname;
const host = nodeHost(join(root, 'dist/wasm'));
const run = async (id, input) => JSON.parse(await host.invoke(id, input));

test('hardening: overflowing number', () => {
  const e = checkJson('{"value":1e400,"to":"mph"}');
  assert.equal(e.error.code, 'INVALID_INPUT');
  assert.equal(e.error.field, '/value');
});

test('hardening: duplicate keys', () => {
  const e = checkJson('{"lat":1,"lat":2}');
  assert.equal(e.error.code, 'INVALID_INPUT');
  assert.match(e.error.message, /"lat" appears twice/);
  assert.equal(checkJson('[{"lat":1},{"lat":2}]'), null);
});

test('hardening: depth, string length, payload size', () => {
  assert.equal(checkJson('['.repeat(33) + ']'.repeat(33)).error.code, 'INVALID_INPUT');
  assert.equal(checkJson('['.repeat(32) + ']'.repeat(32)), null);
  assert.equal(checkJson(JSON.stringify({ s: 'x'.repeat(1_000_001) })).error.code, 'LIMIT_EXCEEDED');
  assert.equal(checkJson('{"a":1}', 3).error.code, 'LIMIT_EXCEEDED');
  assert.equal(checkJson('{"a":').error.code, 'INVALID_INPUT');
});

test('invoke through Wasm: kt-to-mph pair vector', async () => {
  const r = await run('units.speed.kt-to-mph', '{"value":100}');
  assert.equal(r.result.converted.value, 115.07794480235425);
  assert.equal(r.meta.tool, 'units.speed.kt-to-mph');
});

test('unknown tool ids', async () => {
  assert.equal((await run('units.speed.warp', '{}')).error.code, 'UNSUPPORTED');
  assert.equal((await run('nosuch.domain.tool', '{}')).error.code, 'UNSUPPORTED');
  // A known domain whose module is loaded but has no such tool yet.
  assert.equal((await run('aviation.airspeed.cas-to-tas', '{}')).error.code, 'UNSUPPORTED');
});

test('batch keeps order and isolates failures', async () => {
  const out = JSON.parse(
    await host.invokeBatch('units.speed.convert', '[{"value":1,"to":"mph"},{"value":"1 ft","to":"mph"},{"value":2,"to":"mph"}]'),
  );
  assert.deepEqual(out.map((r) => r.ok), [true, false, true]);
});

test('every golden vector passes through Wasm in Node', async () => {
  const dir = join(root, 'core/vectors');
  let n = 0;
  for (const file of readdirSync(dir).filter((f) => f.endsWith('.jsonl'))) {
    const id = file.slice(0, -'.jsonl'.length);
    for (const line of readFileSync(join(dir, file), 'utf8').split('\n').filter(Boolean)) {
      const v = JSON.parse(line);
      if (v.supersededBy) continue;
      const got = await run(id, JSON.stringify(v.input));
      for (const [path, want] of Object.entries(v.expect)) {
        const actual = path.split('.').reduce((o, k) => o?.[k], got);
        if (typeof want === 'number') {
          const t = v.tolerance[path];
          const bound = (t.abs ?? 0) + (t.rel ?? 0) * Math.abs(want);
          assert.ok(Math.abs(actual - want) <= bound, `${id} ${v.id} ${path}: want ${want}, got ${actual}`);
        } else {
          assert.deepEqual(actual, want, `${id} ${v.id} ${path}`);
        }
      }
      n++;
    }
  }
  assert.ok(n > 200, `${n} vectors`);
});

test('worker host: a runaway call times out and the host keeps serving', async () => {
  const { workerHost } = await import('./worker-host.mjs');
  const h = workerHost(join(root, 'dist/wasm'), { timeoutMs: 300 });
  // Warm the worker first so the bound measures the timeout, not module loading.
  await h.invoke('units.speed.kt-to-mph', '{"value":1}');
  const t0 = Date.now();
  const stuck = JSON.parse(await h._call('spin'));
  assert.equal(stuck.error.code, 'LIMIT_EXCEEDED');
  assert.match(stuck.error.message, /300 ms timeout/);
  assert.ok(Date.now() - t0 < 2000);
  const ok = JSON.parse(await h.invoke('units.speed.kt-to-mph', '{"value":100}'));
  assert.equal(ok.result.converted.value, 115.07794480235425);
  await h.close();
});

test('search fixture: every expected tool ranks in the top 3', async () => {
  const { workerHost } = await import('./worker-host.mjs');
  const h = workerHost(join(root, 'dist/wasm'));
  const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
  await h.searchLoad(JSON.stringify(catalog.tools));
  const { cases } = JSON.parse(readFileSync(join(root, 'data/search-fixture.json'), 'utf8'));
  const misses = [];
  for (const { query, expect } of cases) {
    const out = JSON.parse(await h.search(JSON.stringify({ query, limit: 3, includeExperimental: true })));
    const ids = out.result.results.map((r) => r.id);
    if (!ids.includes(expect)) misses.push(`${query} → ${ids.join(', ')} (want ${expect})`);
  }
  await h.close();
  assert.deepEqual(misses, []);
});
