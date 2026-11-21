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
// The package format and the GeoTIFF reading are in grid-package.mjs.
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { packHgrid, readGeoTiff } from './grid-package.mjs';

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

export function build(sourceDirectory) {
  const path = join(sourceDirectory, source.name);
  check(readFileSync(path), source, source.name);
  const output = packHgrid(readGeoTiff(path, ['latitude_offset', 'longitude_offset']));
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
