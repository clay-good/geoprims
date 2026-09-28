import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash, generateKeyPairSync } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import {
  buildNgsTiles,
  buildPgmTiles,
  canonicalJson,
  parseNgsGrid,
  parsePgm,
  signIndex,
  sources,
  verifyIndex,
  verifyNgsCheckpoints,
} from './geoid-tiles.mjs';

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
  assert.deepEqual(Object.keys(sources).filter((name) => name.startsWith('egm')), ['egm2008-2.5', 'egm2008-1']);
  for (const [name, source] of Object.entries(sources).filter(([name]) => name.startsWith('egm'))) {
    assert.match(source.url, /^https:\/\/downloads\.sourceforge\.net\/project\/geographiclib\/geoids-distrib\//, name);
    assert.match(source.archiveSha256, /^[a-f0-9]{64}$/, name);
    assert.match(source.pgmSha256, /^[a-f0-9]{64}$/, name);
    assert.ok(source.archiveBytes > 30_000_000, name);
    assert.ok(source.pgmBytes > source.archiveBytes, name);
    assert.equal(source.version, '2009-08-31');
  }
});

function ngsFixture() {
  const rows = 13;
  const cols = 21;
  const bytes = Buffer.alloc(44 + rows * cols * 4);
  bytes.writeDoubleBE(20, 0);
  bytes.writeDoubleBE(230, 8);
  bytes.writeDoubleBE(1, 16);
  bytes.writeDoubleBE(1, 24);
  bytes.writeInt32BE(rows, 32);
  bytes.writeInt32BE(cols, 36);
  bytes.writeInt32BE(1, 40);
  for (let row = 0; row < rows; row++) {
    for (let col = 0; col < cols; col++) bytes.writeFloatBE(row * 100 + col + 0.25, 44 + 4 * (row * cols + col));
  }
  return bytes;
}

test('the NGS pipeline preserves float32 posts and clamped regional halos', () => {
  const source = ngsFixture();
  const files = new Map();
  const index = buildNgsTiles(source, {
    assetId: 'geoid18',
    region: 'fixture',
    version: 'test',
    sourceSha256: sha256(source),
  }, (name, bytes) => files.set(name, bytes));
  assert.equal(index.tiles.length, 4);
  assert.deepEqual(index.grid, {
    width: 21,
    height: 13,
    south: 20,
    west: -130,
    north: 32,
    east: -110,
    latStep: 1,
    lonStep: 1,
  });
  assert.equal(index.interpolation, 'biquadratic');
  for (const tile of index.tiles) {
    assert.ok(tile.bounds[2] - tile.bounds[0] >= 1, tile.file);
    assert.ok(tile.bounds[3] - tile.bounds[1] >= 1, tile.file);
    assert.equal(sha256(files.get(tile.file)), tile.sha256, tile.file);
  }
  const file = files.get('fixture-n20-w130.ggt');
  const first = file.indexOf(10) + 1;
  const second = file.indexOf(10, first) + 1;
  const meta = JSON.parse(file.subarray(first, second - 1));
  const payload = file.subarray(second);
  assert.equal(meta.encoding, 'float32-be');
  assert.equal(payload.readFloatBE(0), 0.25, 'southwest halo was not clamped');
  assert.equal(payload.readFloatBE(4 * (meta.halo * meta.cols + meta.halo)), 0.25, 'first core post changed');
  const lastFile = files.get('fixture-n30-w120.ggt');
  const lastFirst = lastFile.indexOf(10) + 1;
  const lastSecond = lastFile.indexOf(10, lastFirst) + 1;
  const lastMeta = JSON.parse(lastFile.subarray(lastFirst, lastSecond - 1));
  const lastPayload = lastFile.subarray(lastSecond);
  assert.equal(lastPayload.readFloatBE(4 * (lastMeta.rows * lastMeta.cols - 1)), 1_220.25, 'northeast halo was not clamped');
});

test('GEOID18 sources and the corrected NGS check points are pinned', () => {
  const conus = sources['geoid18-conus'];
  const prvi = sources['geoid18-prvi'];
  assert.deepEqual([conus.bytes, prvi.bytes], [34_297_008, 434_688]);
  for (const source of [conus, prvi]) {
    assert.equal(source.assetId, 'geoid18');
    assert.equal(source.format, 'ngs-float32-be');
    assert.equal(source.version, '2019-11-26');
    assert.match(source.url, /^https:\/\/geodesy\.noaa\.gov\/PC_PROD\/GEOID18\/Format_unix\/g2018[up]0\.bin$/);
    assert.match(source.sha256, /^[a-f0-9]{64}$/);
  }
  assert.equal(conus.checkpoints.length, 8);
  assert.equal(prvi.checkpoints.length, 1);

  const fixture = ngsFixture();
  const grid = parseNgsGrid(fixture);
  verifyNgsCheckpoints(grid, [[22, 233, 203.25]]);
  assert.throws(() => verifyNgsCheckpoints(grid, [[22, 233, 203.251]]), /expected 203\.251 m/);
  assert.throws(() => parseNgsGrid(fixture.subarray(0, -4)), /expected/);
});
