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

test('the page waits before recomputing, so typing is not a request per keystroke', () => {
  const src = readFileSync(new URL('../src/components/ToolApp.svelte', import.meta.url), 'utf8');
  const debounce = /setTimeout\(run, (\d+)\)/.exec(src);
  assert.ok(debounce, 'edits are not debounced');
  const ms = Number(debounce[1]);
  assert.ok(ms >= 100 && ms <= 300, `${ms} ms is not a sensible debounce`);
  // And an edit cancels the pending one rather than queueing another.
  assert.match(src, /clearTimeout\(timer\);\s*\n\s*timer = setTimeout\(run,/);
});
