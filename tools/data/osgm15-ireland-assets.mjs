#!/usr/bin/env node
// Packs OSGM15's two Irish geoid grids, for the Malin Head and Belfast
// height datums, into the grids the core reads.
// Usage: node tools/data/osgm15-ireland-assets.mjs <source-directory>
//
// The source directory must hold OSGM15_Malin.gri and OSGM15_Belfast.gri
// from Ordnance Survey's OSTN15-OSGM15-DevelopersPack.zip (pinned in
// ostn15-assets.mjs). Each is a GRAVSOFT grid: a header line of south, north,
// west and east bounds and the latitude and longitude spacing in degrees, then
// the geoid heights in metres, rows from north to south and each row from west
// to east. Its latitude spacing is printed as 0.013333, which is 48 seconds
// (1/75 degree) rounded, so the package states the spacing in whole seconds.
// Every height is kept to the millimetre as a u16 offset from the grid's
// minimum.
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '../..');
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

/** The grids' extent in arcseconds: 51°N to 56°N, 11°30'W to 5°W, 48" by 72". */
export const SHAPE = { south: 183600, north: 201600, west: -41400, east: -18000, dlat: 48, dlon: 72, rows: 376, cols: 326 };

export const sources = [
  {
    name: 'OSGM15_Malin.gri',
    bytes: 1258171,
    sha256: 'ebca800f99f00f79d4cfa7e07f458a2aed835fc1359a9fdfc30c74f1a60da34f',
    output: { path: 'assets/data/osgm15-ireland/2016/malin.grid', bytes: 245209, sha256: '3dd6297b2240cdbe8261c7b82fcc8d46224a8a6fbd953d9dc828b5cc46fcec3c' },
  },
  {
    name: 'OSGM15_Belfast.gri',
    bytes: 1258171,
    sha256: '4124a0870764fa0fbba81d229bb2c099fcaeaa4a5af31766b6d39359dae64f72',
    output: { path: 'assets/data/osgm15-ireland/2016/belfast.grid', bytes: 245209, sha256: '210040e46cf6d1d64f622d3724d367b520d12cbe094b44b0331fe61edac58c48' },
  },
];

function check(bytes, expected, label) {
  if (bytes.length !== expected.bytes) throw new Error(`${label}: ${bytes.length} bytes, expected ${expected.bytes}`);
  const digest = sha256(bytes);
  if (digest !== expected.sha256) throw new Error(`${label}: sha256 ${digest}, expected ${expected.sha256}`);
}

/** The heights in millimetres, checked against the extent above. */
export function parseGri(text) {
  const t = text.trim().split(/\s+/);
  const head = t.slice(0, 6).map(Number);
  const want = [51, 56, -11.5, -5, 0.013333, 0.02];
  for (const [i, w] of want.entries()) {
    if (Math.abs(head[i] - w) > 1e-9) throw new Error(`OSGM15 grid header ${t.slice(0, 6).join(' ')} is not the Irish grid's`);
  }
  const values = t.slice(6).map((s) => {
    const v = Math.round(Number(s) * 1000);
    if (!Number.isFinite(v) || Math.abs(Number(s) * 1000 - v) > 1e-6) throw new Error(`${s} is not whole millimetres`);
    return v;
  });
  if (values.length !== SHAPE.rows * SHAPE.cols) throw new Error(`${values.length} heights, expected ${SHAPE.rows * SHAPE.cols}`);
  return values;
}

/** `OSGM15IE <south> <north> <west> <east> <dlat> <dlon> <rows> <cols> <base>\n`
 * (arcseconds, then millimetres), then u16 offsets in the source's order. */
export function pack(values) {
  const base = values.reduce((m, v) => Math.min(m, v), Infinity);
  const top = values.reduce((m, v) => Math.max(m, v), -Infinity);
  if (top - base > 0xffff) throw new Error(`the heights span ${top - base} mm, more than a u16 holds`);
  const s = SHAPE;
  const header = Buffer.from(`OSGM15IE ${s.south} ${s.north} ${s.west} ${s.east} ${s.dlat} ${s.dlon} ${s.rows} ${s.cols} ${base}\n`);
  const body = Buffer.alloc(values.length * 2);
  values.forEach((v, i) => body.writeUInt16LE(v - base, i * 2));
  return Buffer.concat([header, body]);
}

export function build(sourceDirectory) {
  return sources.map((source) => {
    const bytes = readFileSync(join(sourceDirectory, source.name));
    check(bytes, source, source.name);
    const output = pack(parseGri(bytes.toString('latin1')));
    check(output, source.output, `${source.name} package`);
    const path = join(root, source.output.path);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, output);
    return [source.output.path, output.length, sha256(output)];
  });
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  if (process.argv.length !== 3) throw new Error('usage: osgm15-ireland-assets.mjs <source-directory>');
  for (const [path, bytes, digest] of build(process.argv[2])) console.log(`${path}: ${bytes} bytes, sha256 ${digest}`);
}
