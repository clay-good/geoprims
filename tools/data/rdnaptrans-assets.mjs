#!/usr/bin/env node
// Packs the two grids RDNAPTRANS™2018 needs into the grid packages the core
// reads: NSGI's rdtrans2018 correction grid, which carries an ETRS89 position
// to real RD Bessel (implementation variant 2), and the NLGEO2018 quasi-geoid,
// which carries an ETRS89 ellipsoidal height to NAP (variant 1).
// Usage: node tools/data/rdnaptrans-assets.mjs <source-directory>
//
// The sources are PROJ-data's copies of NSGI's own grids, pinned below, under
// CC BY 4.0. NSGI's own download page could not be fetched from here (every
// path under nsgi.nl answered 404 to a scripted request), so PROJ-data's
// copies are the ones fetched and their digests are PROJ-data's.
//
// Both grid files hold a coarse parent grid for the Netherlands with its EEZ
// and a dense subgrid for the country, so the correction grid is packed as an
// HGRIDS package of the two and the core uses the finer where it reaches, as
// NSGI says to. Nothing is resampled or rounded.
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { packHgrids, packVgrid, readGeoTiff } from './grid-package.mjs';

const root = join(dirname(fileURLToPath(import.meta.url)), '../..');
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

export const sources = [
  {
    name: 'nl_nsgi_rdtrans2018.tif',
    url: 'https://cdn.proj.org/nl_nsgi_rdtrans2018.tif',
    bytes: 278218,
    sha256: '7653831191b424e715a906468962fc60071cfb71a3186b2a58f098bab8bf41de',
    kind: 'hgrids',
    pages: 2,
    output: { path: 'assets/data/rdnaptrans/2018/rdtrans2018.grid', bytes: 738603, sha256: '7c8cccebe3bf21f259a03435968632af9d9a126f910ea180e82ad650dabced0b' },
  },
  {
    name: 'nl_nsgi_nlgeo2018.tif',
    url: 'https://cdn.proj.org/nl_nsgi_nlgeo2018.tif',
    bytes: 246181,
    sha256: 'f8e32c56bf8940fc3fefbc0e413eb4546633ed598b669c63856cddf8328992c0',
    kind: 'vgrid',
    output: { path: 'assets/data/rdnaptrans/2018/nlgeo2018.grid', bytes: 579155, sha256: '787857bb4ef7747871fae5f2e65bcc3db73718a21bf1b6bf38aca4438e4ad52a' },
  },
];

function check(bytes, expected, label) {
  if (bytes.length !== expected.bytes) {
    throw new Error(`${label}: ${bytes.length} bytes, expected ${expected.bytes}`);
  }
  const digest = sha256(bytes);
  if (digest !== expected.sha256) throw new Error(`${label}: sha256 ${digest}, expected ${expected.sha256}`);
}

export function build(sourceDirectory) {
  const built = [];
  for (const source of sources) {
    const path = join(sourceDirectory, source.name);
    check(readFileSync(path), source, source.name);
    let output;
    if (source.kind === 'hgrids') {
      const pages = [];
      for (let page = 1; page <= source.pages; page++) {
        pages.push(readGeoTiff(`GTIFF_DIR:${page}:${path}`, ['latitude_offset', 'longitude_offset']));
      }
      // Coarsest first, so the package reads parent before subgrid.
      pages.sort((a, b) => b.dlat * b.dlon - a.dlat * a.dlon);
      output = packHgrids(pages);
    } else {
      output = packVgrid(readGeoTiff(path, ['geoid_undulation']));
    }
    check(output, source.output, source.output.path);
    const out = join(root, source.output.path);
    mkdirSync(dirname(out), { recursive: true });
    writeFileSync(out, output);
    built.push({ path: source.output.path, bytes: output.length, sha256: sha256(output) });
  }
  return built;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  if (process.argv.length !== 3) throw new Error('usage: rdnaptrans-assets.mjs <source-directory>');
  for (const b of build(process.argv[2])) console.log(`${b.path}: ${b.bytes} bytes, sha256 ${b.sha256}`);
}
