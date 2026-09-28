#!/usr/bin/env node
// Packs the official NGS NADCON5 release into geoprims regional grids.
// Usage: node tools/data/nadcon5-assets.mjs <source-directory>
//
// The source directory must contain each official NGS latitude/longitude .b
// file and its PROJ-data GeoTIFF. Every input and output is pinned below.
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '../..');
const ngsRoot = 'https://geodesy.noaa.gov/pub/nadcon5/20160901release/Builds';
const projRoot = 'https://cdn.proj.org';
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

const region = (ngsName, outputName, lat, lon, proj, output) => ({
  ngsName,
  outputName,
  lat: {
    name: `nadcon5.${ngsName}.lat.trn.20160901.b`,
    url: `${ngsRoot}/${ngsName}/nadcon5.${ngsName}.lat.trn.20160901.b`,
    ...lat,
  },
  lon: {
    name: `nadcon5.${ngsName}.lon.trn.20160901.b`,
    url: `${ngsRoot}/${ngsName}/nadcon5.${ngsName}.lon.trn.20160901.b`,
    ...lon,
  },
  proj: {
    name: `us_noaa_nadcon5_${outputName}.tif`,
    url: `${projRoot}/us_noaa_nadcon5_${outputName}.tif`,
    ...proj,
  },
  output: {
    path: `assets/data/nadcon5/20160901/${outputName}.grid`,
    ...output,
  },
});

export const sources = [
  region('nad27.nad83_1986.conus', 'nad27_nad83_1986_conus',
    { bytes: 100432, sha256: '62246c39146ea4a4e9d8010150ce8730af5bcb13652b10af5045860880baf9a2' },
    { bytes: 100432, sha256: 'd629ca34ee55062b961e70d39629b267d1ee1ad4dedeac3d0ac967a35f1dcbf8' },
    { bytes: 139914, sha256: 'c7d587e0d0b39b9f46c7de850b9a6a468c17b7139f82a313aa43aa4a64d94fa8' },
    { bytes: 199118, sha256: '283ef0131a6e98dde25d167bde2f9d00ab4a5e93b6c0c1bea58f0e3752881464' }),
  region('nad27.nad83_1986.alaska', 'nad27_nad83_1986_alaska',
    { bytes: 23176, sha256: '1b532bc8e204b07f06ee89abf58fb9899b5e10315a578ec8bf54701a6eff08a1' },
    { bytes: 23176, sha256: '506a239b1393e0dbca1d830ea7c2df8b518232f8d69fc952027807e9c6f6f68f' },
    { bytes: 29603, sha256: '5a71a9bcd73ad0011561d72288e100917bfe6295673e524008b295acbef2447c' },
    { bytes: 45530, sha256: '3787ce1915efae0aaa3776a34c1a5ee3ec2f0f8822cd5e7eea395d960236bbac' }),
  region('ohd.nad83_1986.hawaii', 'ohd_nad83_1986_hawaii',
    { bytes: 509344, sha256: 'bdf0fb6518a5800159c823ad965ea48753fd5d79142d212759d7507e921207b5' },
    { bytes: 509344, sha256: 'b8deb46f89a7ade9850e415275e61b1df545a13561a85ef459556083049df690' },
    { bytes: 384810, sha256: '2f1679167e4343f0585448e9f1afb29f53f2fe6534c1be2075c5482fcba7c33d' },
    { bytes: 1013838, sha256: '15bc6b69c113f8e983e8d3b1614a1c5efa23ca2407517c18d4b2230ab2afe8d9' }),
  region('pr40.nad83_1986.prvi', 'pr40_nad83_1986_prvi',
    { bytes: 6352, sha256: '5c7adaca087512ac830ff825dbc0843e3b6cdc792fbbbcf71ec925c9d5a1be03' },
    { bytes: 6352, sha256: 'e7897df8b2bf5573ffcb4904009846d06b956657bb30d09afa8f993c6d4f11ac' },
    { bytes: 7669, sha256: '91a00353598a30ffc24d3d00e3295da0a95ee378cc3dbcc1d149c6c75ace3931' },
    { bytes: 12265, sha256: 'aa20510d180a7a42756f995d5ec5f827c0aca46126ca0ce6251b14d00947bda0' }),
  region('sp1952.nad83_1986.stpaul', 'sp1952_nad83_1986_stpaul',
    { bytes: 8608, sha256: '93def50c213cfdafa12a06430eef999b89d818aee1567716b5ffccf2d9118805' },
    { bytes: 8608, sha256: '4ba38acd831dfc8d8e795dc70ea383489d6269859ba62c81c27bb3b39d53dd4c' },
    { bytes: 8658, sha256: '7aafd2b9b0ed2bb798da5a8a1ee7a254acf308561e2c2d2684d4ad84d1d966b9' },
    { bytes: 16683, sha256: 'e271fc33e2ae44bb994f89dd9d90b93730842c2b98133ea55f899279267fc488' }),
  region('as62.nad83_1993.as', 'as62_nad83_1993_as',
    { bytes: 219424, sha256: '0a03999a04934b29256633c3ed09327673524afc8f2f5cffd1f7f3bb06797a9c' },
    { bytes: 219424, sha256: '42e379659a8be977d4801acf510a49fc2147eef89e2c16349edafe684aceb2f9' },
    { bytes: 132091, sha256: '917ea0277d17829326eb44e6114c92fe24cfafdedf4e7e4b87e8b985f1dbf0b0' },
    { bytes: 435919, sha256: '9698d5346645f42bbafdedf550d4d73736cd55b17ac9ba5aefade91ccf6e9994' }),
  region('gu63.nad83_1993.guamcnmi', 'gu63_nad83_1993_guamcnmi',
    { bytes: 24736, sha256: '3973fa0798aea931a0d5170d41d678ee471cbd82b903ed3fcd26bd197b550737' },
    { bytes: 24736, sha256: '5ea388c2bb20ab9e7e040b324107153ad730db6d65f6bdca8fe46f0dc871224b' },
    { bytes: 16246, sha256: 'b52ce61a85307ae2efe8229df27fe6169c74bec2da82973e47117f2d06e50291' },
    { bytes: 47498, sha256: '36a7b6b4818f35036e423d868bfe5c262d7649cb36db07d1cb16b5b76f644037' }),
];

function check(bytes, expected, label) {
  if (bytes.length !== expected.bytes) throw new Error(`${label}: ${bytes.length} bytes, expected ${expected.bytes}`);
  const digest = sha256(bytes);
  if (digest !== expected.sha256) throw new Error(`${label}: sha256 ${digest}, expected ${expected.sha256}`);
}

export function parseNgsGrid(bytes) {
  if (bytes.length < 56 || bytes.readInt32BE(0) !== 44 || bytes.readInt32BE(48) !== 44) {
    throw new Error('invalid NGS NADCON5 header record');
  }
  const grid = {
    south: bytes.readDoubleBE(4),
    west: bytes.readDoubleBE(12),
    latStep: bytes.readDoubleBE(20),
    lonStep: bytes.readDoubleBE(28),
    rows: bytes.readInt32BE(36),
    cols: bytes.readInt32BE(40),
    kind: bytes.readInt32BE(44),
    samples: [],
  };
  if (grid.rows < 3 || grid.cols < 3 || grid.kind !== 1 || grid.latStep <= 0 || grid.lonStep <= 0) {
    throw new Error('invalid NGS NADCON5 grid shape');
  }
  let offset = 52;
  const rowBytes = grid.cols * 4;
  for (let row = 0; row < grid.rows; row++) {
    if (offset + rowBytes + 8 > bytes.length || bytes.readInt32BE(offset) !== rowBytes) {
      throw new Error(`invalid NGS NADCON5 row ${row + 1}`);
    }
    offset += 4;
    for (let col = 0; col < grid.cols; col++) grid.samples.push(bytes.readFloatBE(offset + col * 4));
    offset += rowBytes;
    if (bytes.readInt32BE(offset) !== rowBytes) throw new Error(`invalid NGS NADCON5 row ${row + 1}`);
    offset += 4;
  }
  if (offset !== bytes.length) throw new Error('trailing bytes after NGS NADCON5 grid');
  return grid;
}

export function pack(latBytes, lonBytes) {
  const lat = parseNgsGrid(latBytes);
  const lon = parseNgsGrid(lonBytes);
  for (const key of ['south', 'west', 'latStep', 'lonStep', 'rows', 'cols']) {
    if (lat[key] !== lon[key]) throw new Error(`latitude and longitude grids disagree on ${key}`);
  }
  const west = lat.west >= 180 ? lat.west - 360 : lat.west;
  const number = (value) => Number.isInteger(value) ? `${value}.0` : `${value}`;
  const header = Buffer.from(`NADCON5 ${number(west)} ${number(lat.south)} ${number(lat.lonStep)} ${number(lat.latStep)} ${lat.cols} ${lat.rows}\n`);
  const body = Buffer.allocUnsafe(lat.samples.length * 8);
  for (let i = 0; i < lat.samples.length; i++) {
    body.writeFloatLE(lat.samples[i], i * 8);
    body.writeFloatLE(lon.samples[i], i * 8 + 4);
  }
  return Buffer.concat([header, body]);
}

export function build(sourceDirectory) {
  for (const source of sources) {
    const lat = readFileSync(join(sourceDirectory, source.lat.name));
    const lon = readFileSync(join(sourceDirectory, source.lon.name));
    const proj = readFileSync(join(sourceDirectory, source.proj.name));
    check(lat, source.lat, `${source.outputName} NGS latitude`);
    check(lon, source.lon, `${source.outputName} NGS longitude`);
    check(proj, source.proj, `${source.outputName} PROJ-data GeoTIFF`);
    const output = pack(lat, lon);
    check(output, source.output, `${source.outputName} package`);
    const path = join(root, source.output.path);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, output);
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  if (process.argv.length !== 3) throw new Error('usage: nadcon5-assets.mjs <source-directory>');
  build(process.argv[2]);
  console.log(`NADCON5 assets: ${sources.length} official NGS grids verified against PROJ-data and written`);
}
