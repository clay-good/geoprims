// The permalink codec through Wasm: matches the pinned vectors, and interoperates
// with an independent DEFLATE implementation (Node's zlib) in both directions.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { deflateRawSync, inflateRawSync } from 'node:zlib';
import { nodeHost } from './node.mjs';

const root = new URL('../../..', import.meta.url).pathname;
const link = await nodeHost(join(root, 'dist/wasm')).module('link');
const encode = async (req) => JSON.parse(await link.callString('gp_link_encode', JSON.stringify(req)));
const decode = async (frag) => JSON.parse(await link.callString('gp_link_decode', frag));
const { vectors } = JSON.parse(readFileSync(join(root, 'data/fragment-vectors.json'), 'utf8'));

test('Wasm encodings match the pinned vector file byte for byte', async () => {
  for (const v of vectors) {
    const out = await encode({ state: v.state, flags: v.flags });
    assert.equal(out.result.fragment, v.fragment);
  }
});

test('zlib inflates our payloads to the canonical JSON', () => {
  for (const v of vectors) {
    const payload = v.fragment.slice(3).split(';')[0];
    const json = inflateRawSync(Buffer.from(payload, 'base64url')).toString('utf8');
    const sorted = (x) =>
      Array.isArray(x) ? x.map(sorted) : x && typeof x === 'object' ? Object.fromEntries(Object.keys(x).sort().map((k) => [k, sorted(x[k])])) : x;
    assert.equal(json, JSON.stringify(sorted(v.state)), 'canonical JSON: sorted keys, JSON.stringify bytes');
  }
});

test('we inflate zlib output, including dynamic Huffman blocks', async () => {
  const state = { i: { note: 'x'.repeat(2000) + 'yz'.repeat(500), value: '12 kt' }, u: { a: 'b' } };
  for (const level of [0, 1, 9]) {
    const frag = 'v1:' + deflateRawSync(JSON.stringify(state), { level }).toString('base64url');
    const out = await decode(frag);
    assert.deepEqual(out.result.state, state, `level ${level}`);
  }
});

test('every pinned state survives the round trip, flags and all', async () => {
  for (const v of vectors) {
    const out = await decode(v.fragment);
    assert.equal(out.ok, true, v.fragment);
    assert.deepEqual(out.result.state, v.state, v.fragment);
    assert.deepEqual(out.result.flags ?? [], v.flags, `flags of ${v.fragment}`);
  }
});

test('the example fragment carries no state', async () => {
  const out = await decode('example');
  assert.equal(out.ok, true);
  assert.deepEqual(out.result.state ?? {}, {});
});

test('a link from a newer version says so instead of guessing', async () => {
  const newer = await decode('v9:q1bKVLKqVirJV7JSyi3IUNJRKkvMKU0F8gwNDBSyS5RqawE');
  assert.equal(newer.ok, false, JSON.stringify(newer));
  assert.equal(newer.error.code, 'UNSUPPORTED');
  assert.match(newer.error.message, /newer version/i, newer.error.message);
});

test('a damaged fragment is refused rather than half-read', async () => {
  for (const bad of ['v1:not-base64url!!', 'v1:', 'v1:AAAA', 'nonsense', 'v1:q1bKVLKqVirJV7JSyi3IUNJRKkvMKU0F8gwNDBSyS5RqawE;made-up']) {
    const out = await decode(bad);
    assert.equal(out.ok, false, `${bad} should not decode: ${JSON.stringify(out)}`);
  }
});
