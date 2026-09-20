// Runtime tests against the built modules (run `node tools/wasm/build.mjs` first).
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { checkJson } from './harden.mjs';
import { loadModule } from './module.mjs';
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

test('the optional invocation timer observes the ABI without changing the answer', async () => {
  const seen = [];
  const module = await loadModule(readFileSync(join(root, 'dist/wasm/base.wasm')), 'base', {
    onInvoke: (id, ms) => seen.push({ id, ms }),
  });
  const input = '{"value":100}';
  assert.equal(await module.invoke('units.speed.kt-to-mph', input), await host.invoke('units.speed.kt-to-mph', input));
  assert.equal(seen.length, 1);
  assert.equal(seen[0].id, 'units.speed.kt-to-mph');
  assert.ok(Number.isFinite(seen[0].ms) && seen[0].ms >= 0);
  await module.invoke('units.speed.kt-to-mph', '{"value":1e400}');
  assert.equal(seen.length, 1, 'a refused input did not reach the ABI');
});

test('unknown tool ids', async () => {
  assert.equal((await run('units.speed.warp', '{}')).error.code, 'UNSUPPORTED');
  assert.equal((await run('nosuch.domain.tool', '{}')).error.code, 'UNSUPPORTED');
  // A known domain whose module is loaded but has no such tool yet.
  assert.equal((await run('aviation.airspeed.no-such-tool', '{}')).error.code, 'UNSUPPORTED');
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
        if (path.includes('.*.')) {
          // "a.*.b": some element of the list at "a" has `want` at "b".
          const [list, rest] = path.split('.*.');
          const items = list.split('.').reduce((o, k) => o?.[k], got) ?? [];
          assert.ok(items.some((x) => rest.split('.').reduce((o, k) => o?.[k], x) === want), `${id} ${v.id} ${path}`);
          continue;
        }
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

test('worker host: abort stops a spinning call within 100 ms and keeps serving', async () => {
  const { workerHost } = await import('./worker-host.mjs');
  const h = workerHost(join(root, 'dist/wasm'), { timeoutMs: 5_000 });
  try {
    await h.invoke('units.speed.kt-to-mph', '{"value":1}');
    const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
    await h.searchLoad(JSON.stringify(catalog.tools));
    const controller = new AbortController();
    let firstProgress;
    const progress = new Promise((resolve) => { firstProgress = resolve; });
    let updates = 0;
    const stuck = h._callWithOptions('spin', [], {
      signal: controller.signal,
      onProgress: (elapsed) => { updates++; firstProgress(elapsed); },
    });
    assert.ok(await progress >= 200, 'the first progress update was too early');
    const started = performance.now();
    controller.abort();
    assert.equal(await stuck, null, 'a canceled call returned a partial result');
    assert.ok(performance.now() - started < 100, 'cancellation took longer than 100 ms');
    const good = JSON.parse(await h.invoke('units.speed.kt-to-mph', '{"value":100}'));
    assert.equal(good.result.converted.value, 115.07794480235425);
    const found = JSON.parse(await h.search('{"query":"density altitude","limit":1}'));
    assert.equal(found.result.results[0].id, 'aviation.altimetry.density-altitude');
    await new Promise((resolve) => setTimeout(resolve, 300));
    assert.equal(updates, 1, 'progress continued after cancellation');
  } finally {
    await h.close();
  }
});

test('worker host: a queued call can be canceled before it runs', async () => {
  const { workerHost } = await import('./worker-host.mjs');
  const h = workerHost(join(root, 'dist/wasm'), { timeoutMs: 5_000 });
  try {
    const running = new AbortController();
    const queued = new AbortController();
    const stuck = h._callWithOptions('spin', [], { signal: running.signal });
    const next = h.invoke('units.speed.kt-to-mph', '{"value":100}', { signal: queued.signal });
    queued.abort();
    assert.equal(await next, null);
    running.abort();
    assert.equal(await stuck, null);
    const good = JSON.parse(await h.invoke('units.speed.kt-to-mph', '{"value":1}'));
    assert.equal(good.ok, true);
  } finally {
    await h.close();
  }
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
