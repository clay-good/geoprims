#!/usr/bin/env node
// Packs swisstopo's CHENyx06 grid, the LV03 to LV95 distortion of the Swiss
// triangulation, into the horizontal-grid package the core reads
// (gp_geo::hgrid). The package format and the GeoTIFF reading are in
// grid-package.mjs.
// Usage: node tools/data/chenyx06-assets.mjs <source-directory>
//
// The source is PROJ-data's ch_swisstopo_CHENyx06a.tif, swisstopo's CH1903 to
// CH1903+ grid under CC0, pinned below; its digest is PROJ-data's, because
// swisstopo distributes the data set as a FINELTRA triangle file rather than a
// grid. The grid is the approximation swisstopo's own documentation offers
// beside FINELTRA; it reproduces swisstopo's five published EUREF points
// within 8 mm.
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { packHgrid, readGeoTiff } from './grid-package.mjs';

const root = join(dirname(fileURLToPath(import.meta.url)), '../..');
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

export const source = {
  name: 'ch_swisstopo_CHENyx06a.tif',
  url: 'https://cdn.proj.org/ch_swisstopo_CHENyx06a.tif',
  bytes: 1021779,
  sha256: '986d81da9445bdd4cbc3b5e36d8ccdad757d7e09246da68040dd21e53bdfbe40',
  output: { path: 'assets/data/chenyx06/2009/chenyx06a.grid', bytes: 1655209, sha256: 'c7b772302bd786b9a9f30f81a58473381f032e7038992ef5e401ce296483575d' },
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
  check(output, source.output, 'CHENyx06 grid package');
  const out = join(root, source.output.path);
  mkdirSync(dirname(out), { recursive: true });
  writeFileSync(out, output);
  return output;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  if (process.argv.length !== 3) throw new Error('usage: chenyx06-assets.mjs <source-directory>');
  const out = build(process.argv[2]);
  console.log(`CHENyx06 grid: ${out.length} bytes, sha256 ${sha256(out)}`);
}
