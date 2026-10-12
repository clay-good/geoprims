// Runtime tests against the built modules (run `node tools/wasm/build.mjs` first).
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { checkJson } from './harden.mjs';
import {
  checkFileSize,
  MAX_SPARSE_FILE_BYTES,
  MAX_WHOLE_FILE_BYTES,
  rangeSource,
  sourceRange,
} from './files.mjs';
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

test('private file bytes cross the raw ABI and can be released', async () => {
  const module = await loadModule(readFileSync(join(root, 'dist/wasm/raster.wasm')), 'raster');
  module.putFile('local-raster-1', new Uint8Array([0x49, 0x49, 0x2a, 0]));
  module.putFile('local-raster-1', new Uint8Array([0x4d, 0x4d, 0, 0x2a]));
  module.removeFile('local-raster-1');
  module.clearFiles();
  assert.throws(() => module.putFile('../not-private', new Uint8Array()), /file handle/i);
  const base = await loadModule(readFileSync(join(root, 'dist/wasm/base.wasm')), 'base');
  assert.throws(() => base.putFile('not-raster', new Uint8Array()), /does not accept local files/);
  assert.doesNotThrow(() => checkFileSize(MAX_WHOLE_FILE_BYTES));
  assert.throws(() => checkFileSize(MAX_WHOLE_FILE_BYTES + 1), /at most/);
});

function sparseTiledTiff(sample = (value) => value, width = 32) {
  const header = [];
  const u16 = (value) => header.push(value & 255, value >>> 8);
  const u32 = (value) => header.push(value & 255, value >>> 8 & 255, value >>> 16 & 255, value >>> 24);
  const entry = (tag, type, count, value) => { u16(tag); u16(type); u32(count); u32(value); };
  header.push(0x49, 0x49); u16(42); u32(8); u16(11);
  entry(256, 4, 1, width);
  entry(257, 4, 1, 16);
  entry(258, 3, 1, 8);
  entry(259, 3, 1, 1);
  entry(262, 3, 1, 1);
  entry(277, 3, 1, 1);
  entry(284, 3, 1, 1);
  entry(322, 4, 1, 16);
  entry(323, 4, 1, 16);
  entry(324, 4, 2, 146);
  entry(325, 4, 2, 154);
  u32(0);
  u32(100_000); u32(200_000);
  u32(256); u32(256);
  assert.equal(header.length, 162);
  const parts = [
    [0, Uint8Array.from(header)],
    [100_000, Uint8Array.from({ length: 256 }, (_, i) => sample(Math.floor(i / 16) * 32 + i % 16) & 255)],
    [200_000, Uint8Array.from({ length: 256 }, (_, i) => sample(Math.floor(i / 16) * 32 + 16 + i % 16) & 255)],
  ];
  const reads = [];
  return {
    size: 200_256,
    reads,
    slice(start, end) {
      reads.push([start, end]);
      const out = new Uint8Array(end - start);
      for (const [offset, bytes] of parts) {
        const from = Math.max(start, offset);
        const to = Math.min(end, offset + bytes.length);
        if (from < to) out.set(bytes.subarray(from - offset, to - offset), from - start);
      }
      return out;
    },
  };
}

test('range-backed TIFF windows retry only the Rust-requested source pages', async () => {
  const source = sparseTiledTiff();
  await host.openFile('raster.index.ndvi', 'large-cog', source);
  const metadata = JSON.parse(await host.inspectFile('raster.index.ndvi', 'large-cog'));
  assert.equal(metadata.metadata.width, 32);
  assert.equal(metadata.metadata.height, 16);
  assert.deepEqual(source.reads, [[0, 65_536]]);

  source.reads.length = 0;
  const out = JSON.parse(await host.readFileWindow(
    'raster.index.ndvi',
    'large-cog',
    { x: 16, y: 1, width: 2, height: 2 },
    256,
  ));
  assert.equal(out.ok, true, JSON.stringify(out));
  assert.deepEqual(out.raster.samples, [48, 49, 80, 81]);
  assert.deepEqual(source.reads, [[0, 65_536], [196_608, 200_256]]);
  await host.removeFile('raster.index.ndvi', 'large-cog');
});

test('range-backed file sources enforce logical and returned lengths', async () => {
  assert.equal(rangeSource({ size: MAX_SPARSE_FILE_BYTES, slice() {} }).size, MAX_SPARSE_FILE_BYTES);
  assert.throws(() => rangeSource({ size: MAX_SPARSE_FILE_BYTES + 1, slice() {} }), /range-backed file size/i);
  assert.throws(() => rangeSource({ size: 4 }), /provide slice/i);
  await assert.rejects(() => sourceRange({ size: 4, slice: () => new Uint8Array(1) }, 0, 4), /1 bytes; 4 were requested/);
});

test('range-backed file windows run the Rust index kernel without JavaScript pixel math', async () => {
  const left = sparseTiledTiff();
  const right = sparseTiledTiff(() => 16);
  await host.openFile('raster.index.ndvi', 'left-cog', left);
  await host.openFile('raster.index.ndvi', 'right-cog', right);
  const out = JSON.parse(await host.processFileWindow(
    'raster.index.ndvi',
    'left-cog', 0,
    'right-cog', 0,
    { x: 16, y: 1, width: 2, height: 2 },
    256,
  ));
  assert.equal(out.ok, true, JSON.stringify(out));
  assert.deepEqual(out.raster.samples, [0.5, 33 / 65, 2 / 3, 65 / 97]);
  assert.deepEqual(out.raster.valid, [true, true, true, true]);
  assert.deepEqual(left.reads, [[0, 65_536], [196_608, 200_256]]);
  assert.deepEqual(right.reads, [[0, 65_536], [196_608, 200_256]]);
  await host.removeFile('raster.index.ndvi', 'left-cog');
  await host.removeFile('raster.index.ndvi', 'right-cog');
});

test('range-backed index windows compare the complete source grids', async () => {
  await host.openFile('raster.index.ndvi', 'wide-cog', sparseTiledTiff());
  await host.openFile('raster.index.ndvi', 'narrow-cog', sparseTiledTiff((value) => value, 31));
  const out = JSON.parse(await host.processFileWindow(
    'raster.index.ndvi',
    'wide-cog', 0,
    'narrow-cog', 0,
    { x: 16, y: 1, width: 2, height: 2 },
    256,
  ));
  assert.equal(out.error.code, 'GRID_MISMATCH');
  assert.equal(out.error.details.reason, 'dimensions');
  assert.equal(out.error.details.left.width, 32);
  assert.equal(out.error.details.right.width, 31);
  await host.removeFile('raster.index.ndvi', 'wide-cog');
  await host.removeFile('raster.index.ndvi', 'narrow-cog');
});

test('range-backed index windows forward explicit resampling to Rust', async () => {
  await host.openFile('raster.index.ndvi', 'resample-left', sparseTiledTiff());
  await host.openFile('raster.index.ndvi', 'resample-right', sparseTiledTiff(() => 16));
  const out = JSON.parse(await host.processFileWindow(
    'raster.index.ndvi',
    'resample-left', 0,
    'resample-right', 0,
    { x: 0, y: 0, width: 2, height: 2 },
    256,
    { method: 'nearest', target: 'right' },
  ));
  assert.equal(out.error.code, 'INVALID_INPUT');
  assert.match(out.error.message, /affine transform/);
  await host.removeFile('raster.index.ndvi', 'resample-left');
  await host.removeFile('raster.index.ndvi', 'resample-right');
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

test('worker host: abort stops a real long core call within 100 ms and keeps serving', async () => {
  const { workerHost } = await import('./worker-host.mjs');
  const h = workerHost(join(root, 'dist/wasm'), { timeoutMs: 120_000 });
  try {
    await h.invoke('units.speed.kt-to-mph', '{"value":1}');
    const controller = new AbortController();
    let firstProgress;
    const progress = new Promise((resolve) => { firstProgress = resolve; });
    const points = [
      { lat: 40.0, lon: -80.35 },
      { lat: 40.0, lon: -79.55 },
      { lat: 40.5, lon: -79.55 },
      { lat: 40.5, lon: -80.35 },
    ];
    const fill = h.invoke(
      'indexing.h3.polygon-to-cells',
      JSON.stringify({ points, resolution: 10 }),
      { signal: controller.signal, onProgress: firstProgress },
    );
    assert.ok(await progress >= 200, 'the real calculation ended before progress was reported');
    const started = performance.now();
    controller.abort();
    assert.equal(await fill, null, 'a canceled core call returned a partial result');
    assert.ok(performance.now() - started < 100, 'core-call cancellation took longer than 100 ms');
    const good = JSON.parse(await h.invoke('units.speed.kt-to-mph', '{"value":100}'));
    assert.equal(good.result.converted.value, 115.07794480235425);
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
