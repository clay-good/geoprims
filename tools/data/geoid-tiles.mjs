#!/usr/bin/env node
// Splits a global GeographicLib PGM geoid into coarse, independently verified
// tiles. The signed index is canonical JSON so every build signs the same bytes.
import { createHash, createPrivateKey, createPublicKey, sign, verify } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { basename, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

export const sources = {
  'egm2008-2.5': {
    version: '2009-08-31',
    url: 'https://downloads.sourceforge.net/project/geographiclib/geoids-distrib/egm2008-2_5.tar.bz2',
    archiveBytes: 34_927_299,
    archiveSha256: 'd602e13446a4a4a23f39aecfe6a2a0760a1bc6c1b497482c2ebc9f7d513be699',
    member: 'geoids/egm2008-2_5.pgm',
    pgmBytes: 74_667_284,
    pgmSha256: 'fab040a55dfabe782be89a89b2ba7e4a73183513a9813e24a3f80e7b6ed61dbf',
  },
  'egm2008-1': {
    version: '2009-08-31',
    url: 'https://downloads.sourceforge.net/project/geographiclib/geoids-distrib/egm2008-1.tar.bz2',
    archiveBytes: 162_388_303,
    archiveSha256: 'bdb382d0be7ece9142450eacc24b7b7f0889ee3e0ba4f535b04ec383f94c0fb5',
    member: 'geoids/egm2008-1.pgm',
    pgmBytes: 466_603_604,
    pgmSha256: 'b5b3fd38ba630285d8a0dc76071b5e7c730d9d882d8570efee45cfd58729525a',
  },
};

export function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(',')}]`;
  if (value && typeof value === 'object') {
    return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`).join(',')}}`;
  }
  const encoded = JSON.stringify(value);
  if (encoded === undefined) throw new Error('the signed index contains an unsupported value');
  return encoded;
}

function token(bytes, state) {
  while (state.at < bytes.length) {
    const c = bytes[state.at];
    if (c === 35) {
      while (state.at < bytes.length && bytes[state.at] !== 10) state.at++;
    } else if (c === 9 || c === 10 || c === 13 || c === 32) state.at++;
    else break;
  }
  const start = state.at;
  while (state.at < bytes.length && ![9, 10, 13, 32].includes(bytes[state.at])) state.at++;
  if (start === state.at) throw new Error('PGM header ended early');
  return bytes.subarray(start, state.at).toString('ascii');
}

export function parsePgm(bytes) {
  const state = { at: 0 };
  if (token(bytes, state) !== 'P5') throw new Error('geoid source must be a binary P5 PGM');
  const width = Number(token(bytes, state));
  const height = Number(token(bytes, state));
  const max = Number(token(bytes, state));
  if (!Number.isSafeInteger(width) || !Number.isSafeInteger(height) || width < 2 || height < 2 || max !== 65535) {
    throw new Error(`unsupported PGM shape ${width}x${height} max ${max}`);
  }
  if (bytes[state.at] === 13 && bytes[state.at + 1] === 10) state.at += 2;
  else if ([9, 10, 13, 32].includes(bytes[state.at])) state.at++;
  else throw new Error('PGM header needs whitespace before its pixels');
  const expected = width * height * 2;
  if (bytes.length - state.at !== expected) throw new Error(`PGM has ${bytes.length - state.at} pixel bytes; expected ${expected}`);
  const header = bytes.subarray(0, state.at).toString('ascii');
  const value = (name) => new RegExp(`^# ${name} (.+)$`, 'm').exec(header)?.[1];
  const offset = Number(value('Offset'));
  const scale = Number(value('Scale'));
  if (!Number.isFinite(offset) || !Number.isFinite(scale) || scale <= 0) throw new Error('PGM needs finite Offset and positive Scale comments');
  if (value('Origin') !== '90N 0E') throw new Error('only global GeographicLib grids with Origin 90N 0E are supported');
  return { width, height, offset, scale, pixels: bytes.subarray(state.at) };
}

const tileName = (north, west) => `${north >= 0 ? 'n' : 's'}${String(Math.abs(north)).padStart(2, '0')}-${String(west).padStart(3, '0')}.ggt`;

function tileBytes(meta, payload) {
  return Buffer.concat([
    Buffer.from('GEOPRIMS-GEOID-TILE 1\n'),
    Buffer.from(`${JSON.stringify(meta)}\n`),
    payload,
  ]);
}

export function buildPgmTiles(bytes, options, writeTile) {
  const { assetId, version, sourceSha256, tileDegrees = 10, halo = 2 } = options;
  if (!assetId || !version || !/^[a-f0-9]{64}$/.test(sourceSha256 ?? '')) throw new Error('assetId, version, and sourceSha256 are required');
  const sourceDigest = sha256(bytes);
  if (sourceDigest !== sourceSha256) throw new Error(`source sha256 ${sourceDigest}, expected ${sourceSha256}`);
  if (!Number.isInteger(tileDegrees) || tileDegrees < 1 || 180 % tileDegrees || 360 % tileDegrees) {
    throw new Error('tileDegrees must be a whole-degree divisor of 180 and 360');
  }
  if (!Number.isInteger(halo) || halo < 0) throw new Error('halo must be a nonnegative integer');
  const pgm = parsePgm(bytes);
  const lonStep = 360 / pgm.width;
  const latStep = 180 / (pgm.height - 1);
  const cols = tileDegrees / lonStep;
  const rows = tileDegrees / latStep;
  if (!Number.isInteger(cols) || !Number.isInteger(rows)) throw new Error('tile boundaries do not fall on source grid posts');
  const tiles = [];
  for (let north = 90; north > -90; north -= tileDegrees) {
    const coreRow = Math.round((90 - north) / latStep);
    for (let west = 0; west < 360; west += tileDegrees) {
      const coreCol = Math.round(west / lonStep);
      const tileRows = rows + 1 + 2 * halo;
      const tileCols = cols + 1 + 2 * halo;
      const payload = Buffer.allocUnsafe(tileRows * tileCols * 2);
      for (let y = 0; y < tileRows; y++) {
        const sourceRow = Math.max(0, Math.min(pgm.height - 1, coreRow + y - halo));
        for (let x = 0; x < tileCols; x++) {
          const sourceCol = (coreCol + x - halo + pgm.width) % pgm.width;
          const source = 2 * (sourceRow * pgm.width + sourceCol);
          const target = 2 * (y * tileCols + x);
          payload[target] = pgm.pixels[source];
          payload[target + 1] = pgm.pixels[source + 1];
        }
      }
      const name = tileName(north, west);
      const meta = {
        north,
        south: north - tileDegrees,
        west,
        east: west + tileDegrees,
        latStep,
        lonStep,
        rows: tileRows,
        cols: tileCols,
        halo,
        offset: pgm.offset,
        scale: pgm.scale,
        encoding: 'uint16-be',
      };
      const file = tileBytes(meta, payload);
      writeTile(name, file);
      tiles.push({ file: name, bytes: file.length, sha256: sha256(file), bounds: [meta.south, meta.west, meta.north, meta.east] });
    }
  }
  return {
    schemaVersion: 1,
    assetId,
    version,
    source: { file: options.sourceFile ?? 'source.pgm', bytes: bytes.length, sha256: sourceDigest },
    grid: { width: pgm.width, height: pgm.height, latStep, lonStep, offset: pgm.offset, scale: pgm.scale },
    tiling: { tileDegrees, halo, minimumDegrees: 1 },
    tiles,
  };
}

export function signIndex(index, privateKeyPem, keyId) {
  if (!keyId) throw new Error('keyId is required');
  const privateKey = createPrivateKey(privateKeyPem);
  const publicKey = createPublicKey(privateKey);
  const payload = Buffer.from(canonicalJson(index));
  return {
    algorithm: 'Ed25519',
    keyId,
    publicKey: publicKey.export({ type: 'spki', format: 'der' }).toString('base64'),
    index,
    signature: sign(null, payload, privateKey).toString('base64'),
  };
}

export function verifyIndex(signed, expectedPublicKey = signed.publicKey) {
  if (signed.algorithm !== 'Ed25519') return false;
  if (signed.publicKey !== expectedPublicKey) return false;
  const publicKey = createPublicKey({ key: Buffer.from(signed.publicKey, 'base64'), type: 'spki', format: 'der' });
  return verify(null, Buffer.from(canonicalJson(signed.index)), publicKey, Buffer.from(signed.signature, 'base64'));
}

export function build(input, output, privateKeyFile, assetId, version, keyId, sourceSha256) {
  const bytes = readFileSync(input);
  mkdirSync(output, { recursive: true });
  const index = buildPgmTiles(bytes, { assetId, version, sourceFile: basename(input), sourceSha256 }, (name, tile) => writeFileSync(join(output, name), tile));
  const signed = signIndex(index, readFileSync(privateKeyFile), keyId);
  writeFileSync(join(output, 'index.json'), `${JSON.stringify(signed, null, 2)}\n`);
  return signed;
}

export function readPinnedArchive(archive, source) {
  const archiveBytes = readFileSync(archive);
  if (archiveBytes.length !== source.archiveBytes) throw new Error(`archive has ${archiveBytes.length} bytes; expected ${source.archiveBytes}`);
  const digest = sha256(archiveBytes);
  if (digest !== source.archiveSha256) throw new Error(`archive sha256 ${digest}, expected ${source.archiveSha256}`);
  const pgm = execFileSync('tar', ['-xOf', archive, source.member], { maxBuffer: source.pgmBytes + 1 });
  if (pgm.length !== source.pgmBytes) throw new Error(`source PGM has ${pgm.length} bytes; expected ${source.pgmBytes}`);
  const pgmDigest = sha256(pgm);
  if (pgmDigest !== source.pgmSha256) throw new Error(`source PGM sha256 ${pgmDigest}, expected ${source.pgmSha256}`);
  return pgm;
}

export function buildSource(name, archive, output, privateKeyFile, keyId) {
  const source = sources[name];
  if (!source) throw new Error(`unknown geoid source ${name}; choose ${Object.keys(sources).join(' or ')}`);
  const pgm = readPinnedArchive(archive, source);
  mkdirSync(output, { recursive: true });
  const index = buildPgmTiles(pgm, { assetId: name, version: source.version, sourceFile: basename(source.member), sourceSha256: source.pgmSha256 }, (file, tile) => writeFileSync(join(output, file), tile));
  const signed = signIndex(index, readFileSync(privateKeyFile), keyId);
  writeFileSync(join(output, 'index.json'), `${JSON.stringify(signed, null, 2)}\n`);
  return signed;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  if (process.argv.length !== 7) throw new Error('usage: geoid-tiles.mjs SOURCE_NAME ARCHIVE.tar.bz2 OUTPUT_DIR PRIVATE_KEY.pem KEY_ID');
  const signed = buildSource(...process.argv.slice(2));
  console.log(`${signed.index.assetId}: ${signed.index.tiles.length} signed tiles written to ${dirname(join(process.argv[4], 'index.json'))}`);
}
