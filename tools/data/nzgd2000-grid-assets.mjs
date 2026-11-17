#!/usr/bin/env node
// Packs Land Information New Zealand's NZGD1949 to NZGD2000 distortion grid
// into the horizontal-grid package the core reads (gp_geo::hgrid).
// Usage: node tools/data/nzgd2000-grid-assets.mjs <source-directory>
//
// The source is PROJ-data's nz_linz_nzgd2kgrid0005.tif, LINZ's grid under
// CC BY 4.0, pinned below. LINZ's own download page refuses scripted
// requests (403), so PROJ-data's copy is the one fetched; its digest is
// PROJ-data's. GDAL's gdal_translate unpacks the latitude and longitude
// offset bands; the package keeps them as the float32 values they are.
//
// Package: `HGRID <west> <south> <dlon> <dlat> <cols> <rows>\n` in degrees at
// the nodes, then for each node, south row first and west to east, the
// latitude and longitude offsets in arcseconds (east positive) as
// little-endian float32.
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '../..');
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

export const source = {
  name: 'nz_linz_nzgd2kgrid0005.tif',
  url: 'https://cdn.proj.org/nz_linz_nzgd2kgrid0005.tif',
  bytes: 197302,
  sha256: 'd4d9649bfbc55bbbeb7636fa26f300045123d97a3ab01dec3bf2982eae86cbb2',
  output: { path: 'assets/data/nzgd2000-grid/0005/nzgd2kgrid0005.grid', bytes: 159078, sha256: 'bf72216613f47d7f23d78077eb2f07e75e40becbe2634a072d72306e1ecf5c87' },
};

function check(bytes, expected, label) {
  if (bytes.length !== expected.bytes) throw new Error(`${label}: ${bytes.length} bytes, expected ${expected.bytes}`);
  const digest = sha256(bytes);
  if (digest !== expected.sha256) throw new Error(`${label}: sha256 ${digest}, expected ${expected.sha256}`);
}

/** Header and node values from the GeoTIFF, by GDAL. */
export function readGeoTiff(path) {
  const info = JSON.parse(execFileSync('gdalinfo', ['-json', path], { encoding: 'utf8' }));
  const [x0, dx, , y0, , dy] = info.geoTransform;
  const [cols, rows] = info.size;
  const names = info.bands.map((b) => b.description);
  if (names[0] !== 'latitude_offset' || names[1] !== 'longitude_offset') throw new Error(`unexpected bands ${names}`);
  if (info.bands[1].metadata?.['']?.positive_value !== 'east') throw new Error('longitude offsets are not east positive');
  // GDAL always reports the geotransform at the outer corner of the corner
  // pixel, shifting a pixel-is-point file's origin by half a cell to do so,
  // so the nodes are always half a cell in from it.
  const half = 0.5;
  const dir = mkdtempSync(join(tmpdir(), 'hgrid-'));
  execFileSync('gdal_translate', ['-q', '-of', 'ENVI', '-b', '1', '-b', '2', path, join(dir, 'g.bin')]);
  const raw = readFileSync(join(dir, 'g.bin'));
  if (raw.length !== cols * rows * 2 * 4) throw new Error('unexpected raw size');
  const band = (b, r, c) => raw.readFloatLE(((b * rows) + r) * cols * 4 + c * 4);
  return {
    west: x0 + half * dx,
    north: y0 + half * dy,
    dlon: dx,
    dlat: -dy,
    cols,
    rows,
    // Row 0 of the TIFF is the north row.
    at: (northRow, col) => [band(0, northRow, col), band(1, northRow, col)],
  };
}

export function pack(g) {
  const south = g.north - (g.rows - 1) * g.dlat;
  const r = (v) => Number(v.toFixed(9));
  const header = Buffer.from(`HGRID ${r(g.west)} ${r(south)} ${r(g.dlon)} ${r(g.dlat)} ${g.cols} ${g.rows}\n`);
  const body = Buffer.alloc(g.cols * g.rows * 8);
  let i = 0;
  for (let row = g.rows - 1; row >= 0; row--) {
    for (let col = 0; col < g.cols; col++) {
      const [dlat, dlon] = g.at(row, col);
      body.writeFloatLE(dlat, i);
      body.writeFloatLE(dlon, i + 4);
      i += 8;
    }
  }
  return Buffer.concat([header, body]);
}

export function build(sourceDirectory) {
  const path = join(sourceDirectory, source.name);
  check(readFileSync(path), source, source.name);
  const output = pack(readGeoTiff(path));
  check(output, source.output, 'NZGD2000 grid package');
  const out = join(root, source.output.path);
  mkdirSync(dirname(out), { recursive: true });
  writeFileSync(out, output);
  return output;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  if (process.argv.length !== 3) throw new Error('usage: nzgd2000-grid-assets.mjs <source-directory>');
  const out = build(process.argv[2]);
  console.log(`NZGD2000 grid: ${out.length} bytes, sha256 ${sha256(out)}`);
}
