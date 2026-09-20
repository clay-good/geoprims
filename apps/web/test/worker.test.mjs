// The browser compute worker (web/tool-app, platform/compute-core "Execution
// off the main thread"). This is the code every visitor runs, driven here the
// way the page drives it: a message in, an envelope out, and never a silence
// that would leave the answer card marked stale forever.
import { readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const dist = join(web, 'dist');

/** Serves the built site's own files, as the worker's same-origin fetch does. */
globalThis.fetch = async (url) => {
  const path = join(dist, String(url).replace(/^\//, ''));
  if (!existsSync(path)) return { ok: false, status: 404 };
  const bytes = readFileSync(path);
  return {
    ok: true,
    status: 200,
    arrayBuffer: async () => bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength),
    json: async () => JSON.parse(bytes.toString('utf8')),
  };
};

const replies = [];
globalThis.self = { onmessage: null, postMessage: (m) => replies.push(m) };
await import('../src/lib/compute.worker.js');
const handler = globalThis.self.onmessage;

/** Sends one message and waits for its reply. */
async function send(method, args) {
  const seq = replies.length + 1;
  await handler({ data: { seq, method, args } });
  const reply = replies.find((r) => r.seq === seq);
  assert.ok(reply, `no reply to ${method}`);
  return JSON.parse(reply.out);
}

test('the worker answers a real call with the real answer', async () => {
  const out = await send('invoke', ['units.speed.kt-to-mph', JSON.stringify({ value: 100 })]);
  assert.equal(out.ok, true, JSON.stringify(out));
  assert.equal(out.result.converted.value, 115.07794480235425);
});

test('an unknown tool is an envelope, not a thrown error', async () => {
  const out = await send('invoke', ['no.such.tool', '{}']);
  assert.equal(out.ok, false);
  assert.ok(out.error.code, JSON.stringify(out));
});

test('a bad input is refused and the worker keeps serving', async () => {
  const bad = await send('invoke', ['units.speed.kt-to-mph', JSON.stringify({ value: 'not a number' })]);
  assert.equal(bad.ok, false);
  const good = await send('invoke', ['units.speed.kt-to-mph', JSON.stringify({ value: 1 })]);
  assert.equal(good.ok, true, 'the worker stopped serving after a bad input');
});

test('every message gets exactly one reply, so nothing is left waiting', async () => {
  const before = replies.length;
  await send('invoke', ['units.speed.kt-to-mph', JSON.stringify({ value: 2 })]);
  await send('invoke', ['no.such.tool', '{}']);
  assert.equal(replies.length - before, 2);
  assert.deepEqual(
    replies.slice(before).map((r) => typeof r.out),
    ['string', 'string'],
  );
});

test('the search path returns ranked results through the same worker', async () => {
  const out = await send('search', [JSON.stringify({ query: 'knots to mph', limit: 3, includeExperimental: true })]);
  assert.equal(out.ok, true, JSON.stringify(out).slice(0, 200));
  assert.ok(out.result.results.length > 0);
});

test('a failure says what a reader can do, not just that it failed', async () => {
  const out = await send('invoke', ['no.such.tool', '{}']);
  assert.ok(out.error.message.length > 10, out.error.message);
  assert.doesNotMatch(out.error.message, /undefined|\[object/, out.error.message);
});
