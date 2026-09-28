// The asset flow end to end through Wasm: a mock provider that supplies,
// withholds, and corrupts the EGM96 grid (platform-foundation task 4.2).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash, generateKeyPairSync, sign } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { assetProvider, canonicalJson } from './assets.mjs';
import { nodeHost } from './node.mjs';

const root = new URL('../../..', import.meta.url).pathname;
const registry = JSON.parse(readFileSync(join(root, 'assets/registry.json'), 'utf8'));
const grid = readFileSync(join(root, 'assets/data/egm96-15/2009-08-29/egm96-15.pgm'));
const input = '{"lat":16.776,"lon":-3.009}';
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');
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

test('an index failure names the index the browser must evict', async () => {
  const host = nodeHost(join(root, 'dist/wasm'), {
    assets: async (want) => ({
      error: {
        code: 'ASSET_INTEGRITY',
        message: 'The signed index failed verification.',
        asset: { ...want, key: 'index.json' },
      },
    }),
  });
  const r = JSON.parse(await host.invoke('geodesy.geoid.geoid-height', input));
  assert.equal(r.error.code, 'ASSET_INTEGRITY');
  assert.deepEqual(r.error.asset, { id: 'egm96-15', version: '2009-08-29', key: 'index.json' });
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
    if (typeof a.tiling === 'object') {
      assert.equal(a.loadPolicy, 'on-demand-tiled', `${a.id} signed tiles must load on demand`);
      assert.match(a.tiling.index, /^[^/]+\.json$/, `${a.id} needs a safe tile index name`);
      assert.ok(a.files[a.tiling.index], `${a.id} must list its tile index as a file`);
      assert.match(a.tiling.keyId, /^[a-z0-9][a-z0-9._-]+$/, `${a.id} needs a signing key id`);
      assert.match(a.tiling.publicKey, /^[A-Za-z0-9+/]+={0,2}$/, `${a.id} needs a base64 signing public key`);
    }
    if (a.consumers) {
      assert.ok(Array.isArray(a.consumers) && a.consumers.length > 0, `${a.id} consumers`);
      assert.equal(new Set(a.consumers).size, a.consumers.length, `${a.id} repeats a consumer`);
      for (const consumer of a.consumers) assert.ok(['web-assets', 'web-map', 'mcp-server'].includes(consumer), `${a.id} has unknown consumer ${consumer}`);
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

function signedTileFixture() {
  const tile = Buffer.from('one verified geoid tile');
  const { privateKey, publicKey } = generateKeyPairSync('ed25519');
  const publicKeyBase64 = publicKey.export({ type: 'spki', format: 'der' }).toString('base64');
  const index = {
    schemaVersion: 1,
    assetId: 'fixture-geoid',
    version: '2026-09-28',
    tiles: [{ file: 'n00-e000.ggt', bytes: tile.length, sha256: sha256(tile), bounds: [0, 0, 10, 10] }],
  };
  const signed = {
    algorithm: 'Ed25519',
    keyId: 'fixture-2026',
    publicKey: publicKeyBase64,
    index,
    signature: sign(null, Buffer.from(canonicalJson(index)), privateKey).toString('base64'),
  };
  const indexBytes = Buffer.from(`${JSON.stringify(signed)}\n`);
  const fixtureRegistry = {
    assets: [{
      id: index.assetId,
      version: index.version,
      files: { 'index.json': { bytes: indexBytes.length, sha256: sha256(indexBytes) } },
      tiling: { index: 'index.json', keyId: signed.keyId, publicKey: publicKeyBase64 },
      loadPolicy: 'on-demand-tiled',
    }],
  };
  return { tile, signed, indexBytes, registry: fixtureRegistry };
}

test('a pinned signed index authenticates an on-demand tile', async () => {
  const fixture = signedTileFixture();
  const reads = [];
  const verified = [];
  const provide = assetProvider(fixture.registry, async (_id, _version, key) => {
    reads.push(key);
    return key === 'index.json' ? fixture.indexBytes : fixture.tile;
  }, async (asset) => verified.push(asset.key));
  const want = { id: 'fixture-geoid', version: '2026-09-28', key: 'n00-e000.ggt' };
  assert.deepEqual(await provide(want), { bytes: fixture.tile });
  assert.deepEqual(await provide(want), { bytes: fixture.tile });
  assert.deepEqual(reads, ['index.json', 'n00-e000.ggt', 'n00-e000.ggt'], 'the verified index was not cached');
  assert.deepEqual(verified, ['index.json', 'n00-e000.ggt', 'n00-e000.ggt']);
});

test('signed indexes reject tampering, unpinned keys, and corrupt tiles', async () => {
  const tampered = signedTileFixture();
  const body = JSON.parse(tampered.indexBytes);
  body.index.tiles[0].bytes++;
  tampered.indexBytes = Buffer.from(JSON.stringify(body));
  tampered.registry.assets[0].files['index.json'] = {
    bytes: tampered.indexBytes.length,
    sha256: sha256(tampered.indexBytes),
  };
  let provide = assetProvider(tampered.registry, async () => tampered.indexBytes);
  let got = await provide({ id: 'fixture-geoid', version: '2026-09-28', key: 'n00-e000.ggt' });
  assert.equal(got.error.code, 'ASSET_INTEGRITY');
  assert.deepEqual(got.error.asset, { id: 'fixture-geoid', version: '2026-09-28', key: 'index.json' });

  const unpinned = signedTileFixture();
  unpinned.registry.assets[0].tiling.publicKey = generateKeyPairSync('ed25519').publicKey
    .export({ type: 'spki', format: 'der' }).toString('base64');
  provide = assetProvider(unpinned.registry, async () => unpinned.indexBytes);
  got = await provide({ id: 'fixture-geoid', version: '2026-09-28', key: 'n00-e000.ggt' });
  assert.equal(got.error.code, 'ASSET_INTEGRITY');

  const corrupt = signedTileFixture();
  provide = assetProvider(corrupt.registry, async (_id, _version, key) => (
    key === 'index.json' ? corrupt.indexBytes : Buffer.from('corrupt tile')
  ));
  got = await provide({ id: 'fixture-geoid', version: '2026-09-28', key: 'n00-e000.ggt' });
  assert.equal(got.error.code, 'ASSET_INTEGRITY');
  assert.deepEqual(got.error.asset, { id: 'fixture-geoid', version: '2026-09-28', key: 'n00-e000.ggt' });
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
