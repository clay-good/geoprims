#!/usr/bin/env node
// Splits GeographicLib PGM and NGS float32 geoids into coarse, independently
// verified tiles. Canonical JSON makes every build sign the same index bytes.
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
  'geoid18-conus': {
    assetId: 'geoid18',
    region: 'conus',
    format: 'ngs-float32-be',
    version: '2019-11-26',
    url: 'https://geodesy.noaa.gov/PC_PROD/GEOID18/Format_unix/g2018u0.bin',
    bytes: 34_297_008,
    sha256: 'c41654f1c3cc485f302e3bc8e6837fefb1db02b923fb3b6e4ded850c18caeabe',
    checkpoints: [
      [45 + 13 / 60, 360 - (122 + 46 / 60), -23.076],
      [48 + 7 / 60, 360 - (96 + 11 / 60), -27.215],
      [44 + 30 / 60, 360 - (94 + 3 / 60), -28.072],
      [41 + 34 / 60, 360 - (72 + 39 / 60), -29.591],
      [39 + 32 / 60, 360 - (121 + 29 / 60), -27.337],
      [33 + 5 / 60, 360 - (97 + 1 / 60), -26.955],
      [36 + 15 / 60, 360 - (82 + 46 / 60), -31.341],
      [38 + 19 / 60, 360 - (77 + 19 / 60), -32.636],
    ],
  },
  'geoid18-prvi': {
    assetId: 'geoid18',
    region: 'prvi',
    format: 'ngs-float32-be',
    version: '2019-11-26',
    url: 'https://geodesy.noaa.gov/PC_PROD/GEOID18/Format_unix/g2018p0.bin',
    bytes: 434_688,
    sha256: 'e9c5b82348bedd1e3aecf9887e830e6da92f8452cdd42848ff0cb98efee39362',
    checkpoints: [[18 + 27 / 60, 360 - (67 + 24 / 60), -45.673]],
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

export function parseNgsGrid(bytes) {
  if (bytes.length < 44) throw new Error('NGS grid header ended early');
  const latMin = bytes.readDoubleBE(0);
  const lonMin = bytes.readDoubleBE(8);
  const latStep = bytes.readDoubleBE(16);
  const lonStep = bytes.readDoubleBE(24);
  const rows = bytes.readInt32BE(32);
  const cols = bytes.readInt32BE(36);
  const kind = bytes.readInt32BE(40);
  if (![latMin, lonMin, latStep, lonStep].every(Number.isFinite)
    || latMin < -90 || latMin > 90 || lonMin < 0 || lonMin >= 360
    || latStep <= 0 || lonStep <= 0 || rows < 2 || cols < 2 || kind !== 1) {
    throw new Error('unsupported NGS big-endian grid header');
  }
  const expected = 44 + rows * cols * 4;
  if (bytes.length !== expected) throw new Error(`NGS grid has ${bytes.length} bytes; expected ${expected}`);
  const latMax = latMin + latStep * (rows - 1);
  const lonMax = lonMin + lonStep * (cols - 1);
  if (latMax > 90 + 1e-9 || lonMax > 360 + 1e-9) throw new Error('NGS grid bounds are outside latitude or longitude limits');
  return { latMin, lonMin, latStep, lonStep, rows, cols, kind, pixels: bytes.subarray(44) };
}

function ngsGridValue(grid, latitude, longitude) {
  const row = Math.round((latitude - grid.latMin) / grid.latStep);
  const col = Math.round((longitude - grid.lonMin) / grid.lonStep);
  if (row < 0 || row >= grid.rows || col < 0 || col >= grid.cols
    || Math.abs(grid.latMin + row * grid.latStep - latitude) > 1e-8
    || Math.abs(grid.lonMin + col * grid.lonStep - longitude) > 1e-8) {
    throw new Error(`GEOID18 checkpoint ${latitude},${longitude} is not a grid post`);
  }
  return grid.pixels.readFloatBE(4 * (row * grid.cols + col));
}

export function verifyNgsCheckpoints(grid, checkpoints) {
  for (const [latitude, longitude, expected] of checkpoints) {
    const actual = ngsGridValue(grid, latitude, longitude);
    if (actual.toFixed(3) !== expected.toFixed(3)) {
      throw new Error(`GEOID18 checkpoint ${latitude},${longitude} is ${actual.toFixed(3)} m; expected ${expected.toFixed(3)} m`);
    }
  }
}

const tileName = (north, west) => `${north >= 0 ? 'n' : 's'}${String(Math.abs(north)).padStart(2, '0')}-${String(west).padStart(3, '0')}.ggt`;

function tileBytes(meta, payload) {
  return Buffer.concat([
    Buffer.from('GEOPRIMS-GEOID-TILE 1\n'),
    Buffer.from(`${JSON.stringify(meta)}\n`),
    payload,
  ]);
}

const regionalTileName = (region, south, west) => {
  const latitude = `${south >= 0 ? 'n' : 's'}${String(Math.abs(south)).padStart(2, '0')}`;
  const longitude = `${west >= 0 ? 'e' : 'w'}${String(Math.abs(west)).padStart(3, '0')}`;
  return `${region}-${latitude}-${longitude}.ggt`;
};

export function buildNgsTiles(bytes, options, writeTile) {
  const { assetId, region, version, sourceSha256, tileDegrees = 10, halo = 2 } = options;
  if (!assetId || !region || !version || !/^[a-f0-9]{64}$/.test(sourceSha256 ?? '')) {
    throw new Error('assetId, region, version, and sourceSha256 are required');
  }
  const sourceDigest = sha256(bytes);
  if (sourceDigest !== sourceSha256) throw new Error(`source sha256 ${sourceDigest}, expected ${sourceSha256}`);
  if (!Number.isInteger(tileDegrees) || tileDegrees < 1) throw new Error('tileDegrees must be a positive whole number');
  if (!Number.isInteger(halo) || halo < 0) throw new Error('halo must be a nonnegative integer');
  const grid = parseNgsGrid(bytes);
  const wholeDegree = (value) => {
    const rounded = Math.round(value);
    if (Math.abs(value - rounded) > 1e-8) throw new Error('NGS tile coverage must end on whole degrees');
    return rounded;
  };
  const south = wholeDegree(grid.latMin);
  const north = wholeDegree(grid.latMin + grid.latStep * (grid.rows - 1));
  const sourceWest = grid.lonMin > 180 ? grid.lonMin - 360 : grid.lonMin;
  const west = wholeDegree(sourceWest);
  const east = wholeDegree(sourceWest + grid.lonStep * (grid.cols - 1));
  const tiles = [];
  for (let tileSouth = south; tileSouth < north; tileSouth += tileDegrees) {
    const tileNorth = Math.min(north, tileSouth + tileDegrees);
    if (tileNorth - tileSouth < 1) throw new Error('NGS tile latitude span is below 1 degree');
    const coreRow = Math.round((tileSouth - south) / grid.latStep);
    const coreRows = Math.round((tileNorth - tileSouth) / grid.latStep);
    for (let tileWest = west; tileWest < east; tileWest += tileDegrees) {
      const tileEast = Math.min(east, tileWest + tileDegrees);
      if (tileEast - tileWest < 1) throw new Error('NGS tile longitude span is below 1 degree');
      const coreCol = Math.round((tileWest - west) / grid.lonStep);
      const coreCols = Math.round((tileEast - tileWest) / grid.lonStep);
      const rows = coreRows + 1 + 2 * halo;
      const cols = coreCols + 1 + 2 * halo;
      const payload = Buffer.allocUnsafe(rows * cols * 4);
      for (let y = 0; y < rows; y++) {
        const sourceRow = Math.max(0, Math.min(grid.rows - 1, coreRow + y - halo));
        for (let x = 0; x < cols; x++) {
          const sourceCol = Math.max(0, Math.min(grid.cols - 1, coreCol + x - halo));
          const source = 4 * (sourceRow * grid.cols + sourceCol);
          grid.pixels.copy(payload, 4 * (y * cols + x), source, source + 4);
        }
      }
      const name = regionalTileName(region, tileSouth, tileWest);
      const meta = {
        south: tileSouth,
        north: tileNorth,
        west: tileWest,
        east: tileEast,
        latStep: grid.latStep,
        lonStep: grid.lonStep,
        rows,
        cols,
        halo,
        encoding: 'float32-be',
        interpolation: 'biquadratic',
      };
      const file = tileBytes(meta, payload);
      writeTile(name, file);
      tiles.push({ file: name, bytes: file.length, sha256: sha256(file), bounds: [tileSouth, tileWest, tileNorth, tileEast] });
    }
  }
  return {
    schemaVersion: 1,
    assetId,
    region,
    version,
    source: { file: options.sourceFile ?? 'source.bin', bytes: bytes.length, sha256: sourceDigest },
    grid: { width: grid.cols, height: grid.rows, south, west, north, east, latStep: grid.latStep, lonStep: grid.lonStep },
    tiling: { tileDegrees, halo, minimumDegrees: 1 },
    interpolation: 'biquadratic',
    tiles,
  };
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
        let sourceRow = coreRow + y - halo;
        let oppositeMeridian = 0;
        if (sourceRow < 0 || sourceRow >= pgm.height) {
          sourceRow = sourceRow < 0 ? -sourceRow : 2 * (pgm.height - 1) - sourceRow;
          oppositeMeridian = pgm.width / 2;
        }
        for (let x = 0; x < tileCols; x++) {
          let sourceCol = coreCol + x - halo + oppositeMeridian;
          sourceCol = (sourceCol + pgm.width) % pgm.width;
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

export function readPinnedFile(file, source) {
  const bytes = readFileSync(file);
  if (bytes.length !== source.bytes) throw new Error(`source has ${bytes.length} bytes; expected ${source.bytes}`);
  const digest = sha256(bytes);
  if (digest !== source.sha256) throw new Error(`source sha256 ${digest}, expected ${source.sha256}`);
  return bytes;
}

export function buildSource(name, archive, output, privateKeyFile, keyId) {
  const source = sources[name];
  if (!source) throw new Error(`unknown geoid source ${name}; choose ${Object.keys(sources).join(' or ')}`);
  mkdirSync(output, { recursive: true });
  let index;
  if (source.format === 'ngs-float32-be') {
    const bytes = readPinnedFile(archive, source);
    const grid = parseNgsGrid(bytes);
    verifyNgsCheckpoints(grid, source.checkpoints);
    index = buildNgsTiles(bytes, {
      assetId: source.assetId,
      region: source.region,
      version: source.version,
      sourceFile: basename(archive),
      sourceSha256: source.sha256,
    }, (file, tile) => writeFileSync(join(output, file), tile));
  } else {
    const pgm = readPinnedArchive(archive, source);
    index = buildPgmTiles(pgm, { assetId: name, version: source.version, sourceFile: basename(source.member), sourceSha256: source.pgmSha256 }, (file, tile) => writeFileSync(join(output, file), tile));
  }
  const signed = signIndex(index, readFileSync(privateKeyFile), keyId);
  writeFileSync(join(output, 'index.json'), `${JSON.stringify(signed, null, 2)}\n`);
  return signed;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  if (process.argv.length !== 7) throw new Error('usage: geoid-tiles.mjs SOURCE_NAME SOURCE_FILE OUTPUT_DIR PRIVATE_KEY.pem KEY_ID');
  const signed = buildSource(...process.argv.slice(2));
  console.log(`${signed.index.assetId}: ${signed.index.tiles.length} signed tiles written to ${dirname(join(process.argv[4], 'index.json'))}`);
}
