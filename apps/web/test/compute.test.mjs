// Live recompute (web/tool-app "stale results"). A person typing produces a
// request per keystroke; only the newest may reach the answer card. An older
// reply arriving late has to be dropped, not shown, or the card would settle
// on a value for inputs the page no longer holds.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

/** A worker whose replies are delivered by hand, in whatever order. */
class StubWorker {
  constructor() {
    this.sent = [];
    this.onmessage = null;
    StubWorker.last = this;
  }
  postMessage(msg) {
    this.sent.push(msg);
  }
  terminate() {
    this.terminated = true;
  }
  /** Replies to the nth message sent, newest-last. */
  reply(index, out) {
    this.onmessage({ data: { seq: this.sent[index].seq, out: JSON.stringify(out) } });
  }
}

globalThis.Worker = StubWorker;
const compute = await import('../src/lib/compute.js');

test('an older reply arriving late is dropped, not shown', async () => {
  const first = compute.invoke('a.b.c', { v: 1 });
  const old = StubWorker.last;
  const second = compute.invoke('a.b.c', { v: 2 });
  const current = StubWorker.last;
  assert.equal(old.terminated, true);
  current.reply(0, { ok: true, result: { v: 2 } });
  old.reply(0, { ok: true, result: { v: 1 } });
  assert.deepEqual(await second, { ok: true, result: { v: 2 } });
  assert.equal(await first, null, 'the superseded reply must not resolve to a result');
});

test('replies for different keys both reach their callers', async () => {
  const first = compute.invoke('a.b.c', { v: 3 }, 'first');
  const second = compute.invoke('a.b.c', { v: 4 }, 'second');
  const w = StubWorker.last;
  const [i, j] = [w.sent.length - 2, w.sent.length - 1];
  w.reply(i, { ok: true, result: { v: 3 } });
  w.reply(j, { ok: true, result: { v: 4 } });
  assert.deepEqual(await first, { ok: true, result: { v: 3 } });
  assert.deepEqual(await second, { ok: true, result: { v: 4 } });
});

test('different kinds of request do not supersede each other', async () => {
  const answer = compute.invoke('a.b.c', { v: 5 });
  const link = compute.encodeLink({ i: { v: 5 } });
  const found = compute.search({ query: 'x' });
  const w = StubWorker.last;
  const n = w.sent.length;
  w.reply(n - 3, { ok: true, result: { v: 5 } });
  w.reply(n - 2, { ok: true, result: { fragment: 'v1:abc' } });
  w.reply(n - 1, { ok: true, result: { results: [] } });
  assert.deepEqual(await answer, { ok: true, result: { v: 5 } });
  assert.deepEqual(await link, { ok: true, result: { fragment: 'v1:abc' } });
  assert.deepEqual(await found, { ok: true, result: { results: [] } });
});

test('the worker is reused when no call needs interruption', async () => {
  const before = StubWorker.last;
  const answer = compute.invoke('a.b.c', { v: 6 });
  assert.equal(StubWorker.last, before, 'a second worker was started');
  before.reply(before.sent.length - 1, { ok: true, result: { v: 6 } });
  await answer;
});

test('warm-up asks the worker to compile the tool module', async () => {
  const w = StubWorker.last;
  const warmed = compute.warm('aviation.wind.runway-components');
  const i = w.sent.length - 1;
  assert.deepEqual(w.sent[i], { seq: w.sent[i].seq, method: 'warm', args: ['aviation.wind.runway-components'] });
  w.reply(i, { ok: true });
  assert.deepEqual(await warmed, { ok: true });
});

test('private raster bytes are restored after cancellation replaces the worker', async () => {
  const bytes = new Uint8Array([0x49, 0x49, 0x2a, 0]);
  const original = StubWorker.last;
  const stored = compute.putFile('raster.index.ndvi', 'raster-1', bytes);
  const put = original.sent.length - 1;
  original.reply(put, { ok: true });
  assert.deepEqual(await stored, { ok: true });

  const running = compute.invoke('raster.index.ndvi', { file: 'raster-1' });
  compute.cancel();
  assert.equal(await running, null);

  const warmed = compute.warm('raster.index.ndvi');
  const replacement = StubWorker.last;
  assert.notEqual(replacement, original);
  assert.deepEqual(replacement.sent[0], {
    seq: 0,
    method: 'putFile',
    args: ['raster.index.ndvi', 'raster-1', bytes],
  });
  replacement.reply(1, { ok: true });
  await warmed;

  const removed = compute.removeFile('raster.index.ndvi', 'raster-1');
  replacement.reply(2, { ok: true });
  assert.deepEqual(await removed, { ok: true });
});

test('clearing one raster tool releases files for the whole raster module', async () => {
  let current = StubWorker.last;
  const stored = compute.putFile('raster.index.ndvi', 'shared-raster', new Uint8Array([1]));
  current.reply(current.sent.length - 1, { ok: true });
  await stored;
  const cleared = compute.clearFiles('raster.index.nbr');
  current.reply(current.sent.length - 1, { ok: true });
  await cleared;

  const running = compute.invoke('raster.index.ndvi', { file: 'shared-raster' });
  compute.cancel();
  assert.equal(await running, null);
  const warmed = compute.warm('raster.index.ndvi');
  current = StubWorker.last;
  assert.equal(current.sent[0].method, 'warm', 'a cleared file was restored');
  current.reply(0, { ok: true });
  await warmed;
});

test('the page waits before recomputing, so typing is not a request per keystroke', () => {
  const src = readFileSync(new URL('../src/components/ToolApp.svelte', import.meta.url), 'utf8');
  const debounce = /setTimeout\(run, (\d+)\)/.exec(src);
  assert.ok(debounce, 'edits are not debounced');
  const ms = Number(debounce[1]);
  assert.ok(ms >= 100 && ms <= 300, `${ms} ms is not a sensible debounce`);
  // And an edit cancels the pending one rather than queueing another.
  assert.match(src, /clearTimeout\(timer\);\s*\n\s*timer = setTimeout\(run,/);
});
