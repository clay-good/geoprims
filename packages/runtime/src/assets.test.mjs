// The asset flow end to end through Wasm: a mock provider that supplies,
// withholds, and corrupts the EGM96 grid (platform-foundation task 4.2).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { assetProvider } from './assets.mjs';
import { nodeHost } from './node.mjs';

const root = new URL('../../..', import.meta.url).pathname;
const registry = JSON.parse(readFileSync(join(root, 'assets/registry.json'), 'utf8'));
const grid = readFileSync(join(root, 'assets/data/egm96-15/2009-08-29/egm96-15.pgm'));
const input = '{"lat":16.776,"lon":-3.009}';
const run = async (read) =>
  JSON.parse(await nodeHost(join(root, 'dist/wasm'), { assets: assetProvider(registry, read) }).invoke('geodesy.geoid.geoid-height', input));

test('a supplied, verified asset is used and echoed in meta.assets', async () => {
  let reads = 0;
  const host = nodeHost(join(root, 'dist/wasm'), {
    assets: assetProvider(registry, async () => {
      reads++;
      return grid;
    }),
  });
  const r = JSON.parse(await host.invoke('geodesy.geoid.geoid-height', input));
  assert.equal(r.ok, true);
  assert.ok(Math.abs(r.result.geoid_height.value - 28.7079) < 1e-4);
  assert.deepEqual(r.meta.assets, [{ id: 'egm96-15', version: '2009-08-29' }]);
  await host.invoke('geodesy.geoid.geoid-height', input);
  assert.equal(reads, 1, 'the asset is read once and kept');
});

test('a withheld asset is ASSET_UNAVAILABLE naming the file', async () => {
  const r = await run(async () => null);
  assert.equal(r.error.code, 'ASSET_UNAVAILABLE');
  assert.deepEqual(r.error.asset, { id: 'egm96-15', version: '2009-08-29', key: 'egm96-15.pgm' });
});

test('a corrupted asset is ASSET_INTEGRITY and never used', async () => {
  const bad = Uint8Array.from(grid);
  bad[1000] ^= 1;
  const r = await run(async () => bad);
  assert.equal(r.error.code, 'ASSET_INTEGRITY');
});

test('the default Node host finds the repository assets', async () => {
  const r = JSON.parse(await nodeHost(join(root, 'dist/wasm')).invoke('geodesy.geoid.geoid-height', input));
  assert.equal(r.ok, true, JSON.stringify(r));
});

test('every registry entry is complete and every file matches its digest', async () => {
  const { createHash } = await import('node:crypto');
  const identities = new Set();
  for (const a of registry.assets) {
    for (const k of ['id', 'version', 'title', 'issuer', 'license', 'attribution', 'sourceUrl', 'retrievedAt', 'tiling', 'loadPolicy']) {
      assert.ok(a[k], `${a.id} needs ${k}`);
    }
    assert.match(a.id, /^[a-z0-9]+(?:-[a-z0-9]+)*$/, `${a.id} is not a stable asset id`);
    const identity = `${a.id}@${a.version}`;
    assert.ok(!identities.has(identity), `${identity} is duplicated`);
    identities.add(identity);
    assert.ok(['bundled', 'on-demand', 'on-demand-tiled', 'offline-pack-only'].includes(a.loadPolicy), `${a.id} has load policy ${a.loadPolicy}`);
    assert.equal(a.loadPolicy === 'bundled', Boolean(a.bundledIn), `${a.id} bundledIn must match its load policy`);
    if (a.consumers) {
      assert.ok(Array.isArray(a.consumers) && a.consumers.length > 0, `${a.id} consumers`);
      assert.equal(new Set(a.consumers).size, a.consumers.length, `${a.id} repeats a consumer`);
      for (const consumer of a.consumers) assert.ok(['web-map', 'mcp-server'].includes(consumer), `${a.id} has unknown consumer ${consumer}`);
    }
    assert.ok(Object.keys(a.files).length > 0, `${a.id} lists no files`);
    for (const [file, meta] of Object.entries(a.files)) {
      assert.ok(file && !file.includes('/') && file !== '.' && file !== '..', `${a.id} has unsafe file name ${file}`);
      assert.match(meta.sha256, /^[a-f0-9]{64}$/, `${a.id}/${file} sha256`);
      assert.ok(Number.isSafeInteger(meta.bytes) && meta.bytes > 0, `${a.id}/${file} bytes`);
      const path = a.bundledIn ? join(root, a.bundledIn) : join(root, 'assets/data', a.id, a.version, file);
      const bytes = readFileSync(path);
      assert.equal(bytes.length, meta.bytes, `${a.id}/${file} size`);
      assert.equal(createHash('sha256').update(bytes).digest('hex'), meta.sha256, `${a.id}/${file} digest`);
    }
  }
});

test('the registry and the catalog agree about which datasets exist', () => {
  const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
  const registered = new Set(registry.assets.map((a) => a.id));
  const declared = new Set(catalog.tools.flatMap((t) => t.assets ?? []));
  const problems = [];
  for (const id of declared) if (!registered.has(id)) problems.push(`${id} is used by a tool but not in the registry`);
  // An unused row is dead weight a reader would see on /licenses/.
  for (const a of registry.assets) {
    if (!declared.has(a.id) && !a.consumers?.length) problems.push(`${a.id} is registered but no tool or platform surface uses it`);
  }
  assert.deepEqual(problems, []);
  assert.ok(registered.size > 0, 'the registry is empty');
});

test('every registry row carries what the licenses page has to show', () => {
  for (const a of registry.assets) {
    assert.match(a.sourceUrl, /^https:\/\//, `${a.id} needs an https source`);
    assert.match(a.retrievedAt, /^\d{4}-\d{2}-\d{2}$/, `${a.id} needs an ISO retrieval date`);
    assert.ok(['none', 'tiled'].includes(a.tiling) || typeof a.tiling === 'string', `${a.id} tiling`);
    // Attribution is what the licence asks be shown, so it cannot be a stub.
    assert.ok(a.attribution.length > 20, `${a.id}'s attribution is too short to be real`);
  }
});
