import assert from 'node:assert/strict';
import { test } from 'node:test';

const workers = [];
globalThis.Worker = class {
  constructor() {
    this.generation = workers.length;
    this.terminated = false;
    workers.push(this);
  }
  postMessage(message) {
    if (this.generation === 0 || message.args[0] === 'slow') return;
    queueMicrotask(() => this.onmessage?.({ data: {
      seq: message.seq,
      out: JSON.stringify({ ok: true, result: { method: message.method } }),
    } }));
  }
  terminate() {
    this.terminated = true;
  }
};

const compute = await import('../src/lib/compute.js');

test('cancel stops a running call within 100 ms, drops its answer, and replays other work', async () => {
  const slow = compute.invoke('slow', {}, 'invoke');
  const search = compute.search({ query: 'wind' });
  const started = performance.now();
  compute.cancel('invoke');
  assert.equal(await slow, null);
  assert.ok(performance.now() - started < 100, 'cancel took longer than 100 ms');
  assert.equal(workers[0].terminated, true);
  assert.equal((await search).result.method, 'search', 'the pending search was not replayed');
  assert.equal((await compute.invoke('fast', {}, 'invoke')).result.method, 'invoke');
});

test('a new invocation interrupts the prior invocation with the same key', async () => {
  const slow = compute.invoke('slow', {}, 'invoke');
  const fast = compute.invoke('fast', {}, 'invoke');
  assert.equal(await slow, null);
  assert.equal((await fast).result.method, 'invoke');
});

test('a running call reports elapsed time every 250 ms and stops reporting after cancellation', async () => {
  let count = 0;
  let firstProgress;
  const first = new Promise((resolve) => { firstProgress = resolve; });
  const slow = compute.invoke('slow', {}, 'invoke', (elapsed) => {
    count++;
    firstProgress(elapsed);
  });
  assert.ok(await first >= 200, 'the first elapsed-time update was too early');
  compute.cancel('invoke');
  assert.equal(await slow, null);
  await new Promise((resolve) => setTimeout(resolve, 300));
  assert.equal(count, 1, 'a canceled call kept reporting progress');
});
