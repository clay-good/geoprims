import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash, generateKeyPairSync } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { buildPgmTiles, canonicalJson, parsePgm, signIndex, sources, verifyIndex } from './geoid-tiles.mjs';

const root = new URL('../..', import.meta.url).pathname;
const source = readFileSync(join(root, 'assets/data/egm96-15/2009-08-29/egm96-15.pgm'));
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');
const sourceSha256 = sha256(source);
const options = { assetId: 'egm96-15', version: '2009-08-29', sourceSha256 };

test('the EGM96 pipeline makes coarse exact tiles with verified digests and a signed index', () => {
  const files = new Map();
  const index = buildPgmTiles(
    source,
    { ...options, sourceFile: 'egm96-15.pgm' },
    (name, bytes) => files.set(name, bytes),
  );
  assert.equal(index.tiles.length, 18 * 36);
  assert.equal(files.size, index.tiles.length);
  for (const tile of index.tiles) {
    assert.ok(tile.bounds[2] - tile.bounds[0] >= 1, tile.file);
    assert.ok(tile.bounds[3] - tile.bounds[1] >= 1, tile.file);
    assert.equal(files.get(tile.file).length, tile.bytes, tile.file);
    assert.equal(sha256(files.get(tile.file)), tile.sha256, tile.file);
  }
  const { privateKey } = generateKeyPairSync('ed25519');
  const signed = signIndex(index, privateKey.export({ type: 'pkcs8', format: 'pem' }), 'test-only');
  assert.equal(verifyIndex(signed), true);
  const other = generateKeyPairSync('ed25519').publicKey.export({ type: 'spki', format: 'der' }).toString('base64');
  assert.equal(verifyIndex(signed, other), false, 'an unpinned signing key was accepted');
  signed.index.tiles[0].bytes++;
  assert.equal(verifyIndex(signed), false, 'editing the index did not break its signature');
});

test('tile payloads preserve source posts exactly, including wrap and polar halos', () => {
  const files = new Map();
  buildPgmTiles(source, options, (name, bytes) => files.set(name, bytes));
  const pgm = parsePgm(source);
  for (const [name, file] of files) {
    const first = file.indexOf(10) + 1;
    const second = file.indexOf(10, first) + 1;
    const meta = JSON.parse(file.subarray(first, second - 1));
    const payload = file.subarray(second);
    const coreRow = Math.round((90 - meta.north) / meta.latStep);
    const coreCol = Math.round(meta.west / meta.lonStep);
    for (const [y, x] of [[0, 0], [meta.halo, meta.halo], [meta.rows - 1, meta.cols - 1]]) {
      const sourceRow = Math.max(0, Math.min(pgm.height - 1, coreRow + y - meta.halo));
      const sourceCol = (coreCol + x - meta.halo + pgm.width) % pgm.width;
      assert.equal(payload.readUInt16BE(2 * (y * meta.cols + x)), pgm.pixels.readUInt16BE(2 * (sourceRow * pgm.width + sourceCol)), `${name} ${y},${x}`);
    }
  }
});

test('the privacy floor and source shape are enforced', () => {
  assert.equal(canonicalJson({ z: 1, a: { y: 2, b: 3 } }), '{"a":{"b":3,"y":2},"z":1}');
  assert.throws(() => buildPgmTiles(source, { ...options, tileDegrees: 0.5 }, () => {}), /whole-degree/);
  assert.throws(() => buildPgmTiles(source, { ...options, sourceSha256: '0'.repeat(64) }, () => {}), /source sha256/);
  const bad = Buffer.from(source.subarray(0, -2));
  assert.throws(() => parsePgm(bad), /pixel bytes/);
});

test('both official EGM2008 archives and extracted grids are pinned', () => {
  assert.deepEqual(Object.keys(sources), ['egm2008-2.5', 'egm2008-1']);
  for (const [name, source] of Object.entries(sources)) {
    assert.match(source.url, /^https:\/\/downloads\.sourceforge\.net\/project\/geographiclib\/geoids-distrib\//, name);
    assert.match(source.archiveSha256, /^[a-f0-9]{64}$/, name);
    assert.match(source.pgmSha256, /^[a-f0-9]{64}$/, name);
    assert.ok(source.archiveBytes > 30_000_000, name);
    assert.ok(source.pgmBytes > source.archiveBytes, name);
    assert.equal(source.version, '2009-08-31');
  }
});
