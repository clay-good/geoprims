#!/usr/bin/env node
// Packs AdV's BeTA2007 grid, the national DHDN to ETRS89 transformation for
// Germany, into the horizontal-grid package the core reads (gp_geo::hgrid).
// The package format and the GeoTIFF reading are in grid-package.mjs.
// Usage: node tools/data/beta2007-assets.mjs <source-directory>
//
// The source is PROJ-data's de_adv_BETA2007.tif, AdV's BETA2007.gsb converted
// to a Geodetic TIFF Grid, pinned below. AdV allows and welcomes free
// redistribution; its own download pages did not resolve from here, so
// PROJ-data's copy is the one fetched and its digest is PROJ-data's.
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { packHgrid, readGeoTiff } from './grid-package.mjs';

const root = join(dirname(fileURLToPath(import.meta.url)), '../..');
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

export const source = {
  name: 'de_adv_BETA2007.tif',
  url: 'https://cdn.proj.org/de_adv_BETA2007.tif',
  bytes: 24379,
  sha256: '46e681fcc7d022dde1db1f9d0a3426a9bfb1d4a151af69a81b3c30104c9388e2',
  output: { path: 'assets/data/beta2007/2009/beta2007.grid', bytes: 41706, sha256: '40fc5fae21874c9ca1904b9b5daac85822aff4228f1f4da6d02095683b3855c9' },
};

function check(bytes, expected, label) {
  if (bytes.length !== expected.bytes) {
    throw new Error(`${label}: ${bytes.length} bytes, expected ${expected.bytes}`);
  }
  const digest = sha256(bytes);
  if (digest !== expected.sha256) throw new Error(`${label}: sha256 ${digest}, expected ${expected.sha256}`);
}

export function build(sourceDirectory) {
  const path = join(sourceDirectory, source.name);
  check(readFileSync(path), source, source.name);
  const output = packHgrid(readGeoTiff(path, ['latitude_offset', 'longitude_offset']));
  check(output, source.output, 'BeTA2007 grid package');
  const out = join(root, source.output.path);
  mkdirSync(dirname(out), { recursive: true });
  writeFileSync(out, output);
  return output;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  if (process.argv.length !== 3) throw new Error('usage: beta2007-assets.mjs <source-directory>');
  const out = build(process.argv[2]);
  console.log(`BeTA2007 grid: ${out.length} bytes, sha256 ${sha256(out)}`);
}
