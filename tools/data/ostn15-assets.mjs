#!/usr/bin/env node
// Packs Ordnance Survey's native OSTN15/OSGM15 data file into the geoprims
// grid the core reads.
// Usage: node tools/data/ostn15-assets.mjs <source-directory>
//
// The source directory must hold OSTN15_OSGM15_DataFile.txt from Ordnance
// Survey's OSTN15-OSGM15-DevelopersPack.zip. The data file and the zip it
// comes from are pinned below, and so is the output.
//
// The data file gives, at every node of a 1 km grid 700 km by 1,250 km, the
// ETRS89 to OSGB36 easting and northing shifts and the OSGM15 geoid height, all
// to the millimetre, and the node's height-datum flag. The package keeps every
// value exactly: each field is stored in millimetres as an unsigned 16-bit
// offset from that field's minimum, which the header carries, so a node is 7
// bytes and the grid 6.1 MB rather than the source's 41 MB of text.
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '../..');
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

export const COLS = 701;
export const ROWS = 1251;

export const source = {
  zip: {
    name: 'OSTN15-OSGM15-DevelopersPack.zip',
    url: 'https://www.ordnancesurvey.co.uk/documents/resources/OSTN15-OSGM15-DevelopersPack.zip',
    bytes: 15499083,
    sha256: '67069a6d6efe74f637249366f400f3192c03853c78d5a9c3eac2c78ede88364d',
  },
  data: {
    name: 'OSTN15_OSGM15_DataFile.txt',
    bytes: 41095327,
    sha256: '2bb62cf2a1d000f5ce0bbead91eebdf1c72902cc94c7e76ca368e46ff6db5221',
  },
  output: {
    path: 'assets/data/ostn15/2016/ostn15-osgm15.grid',
    bytes: 6138692,
    sha256: '193b03771f757a1834b52a79220f9762d605a0098b19d8f5ffc0df1849d2b017',
  },
};

function check(bytes, expected, label) {
  if (bytes.length !== expected.bytes) throw new Error(`${label}: ${bytes.length} bytes, expected ${expected.bytes}`);
  const digest = sha256(bytes);
  if (digest !== expected.sha256) throw new Error(`${label}: sha256 ${digest}, expected ${expected.sha256}`);
}

/** The data file's rows, checked against the record order OS documents. */
export function parseDataFile(text) {
  const lines = text.split(/\r?\n/).filter((l) => l.trim() !== '');
  if (!lines[0].startsWith('Point_ID,')) throw new Error('the OSTN15 data file starts with its header line');
  const nodes = [];
  for (let i = 1; i < lines.length; i++) {
    const p = lines[i].split(',');
    const record = i - 1;
    if (p.length !== 7 || Number(p[0]) !== record + 1
      || Number(p[1]) !== (record % COLS) * 1000 || Number(p[2]) !== Math.floor(record / COLS) * 1000) {
      throw new Error(`OSTN15 record ${record + 1} is out of order: ${lines[i]}`);
    }
    const mm = (s) => {
      const v = Math.round(Number(s) * 1000);
      if (!Number.isFinite(v) || Math.abs(Number(s) * 1000 - v) > 1e-6) throw new Error(`OSTN15 record ${record + 1}: ${s} is not whole millimetres`);
      return v;
    };
    nodes.push([mm(p[3]), mm(p[4]), mm(p[5]), Number(p[6])]);
  }
  if (nodes.length !== COLS * ROWS) throw new Error(`OSTN15 has ${nodes.length} records, expected ${COLS * ROWS}`);
  return nodes;
}

/** `OSTN15 <cols> <rows> <se0> <sn0> <sg0>\n`, then per node, in record order,
 * three little-endian u16 offsets in millimetres from those minima and a u8 flag. */
export function pack(nodes) {
  const base = [0, 1, 2].map((k) => nodes.reduce((m, n) => Math.min(m, n[k]), Infinity));
  for (const [k, b] of base.entries()) {
    const top = nodes.reduce((m, n) => Math.max(m, n[k]), -Infinity);
    if (top - b > 0xffff) throw new Error(`OSTN15 field ${k} spans ${top - b} mm, more than a u16 holds`);
  }
  const header = Buffer.from(`OSTN15 ${COLS} ${ROWS} ${base[0]} ${base[1]} ${base[2]}\n`);
  const body = Buffer.alloc(nodes.length * 7);
  nodes.forEach(([se, sn, sg, flag], i) => {
    if (!Number.isInteger(flag) || flag < 0 || flag > 255) throw new Error(`OSTN15 record ${i + 1}: flag ${flag}`);
    body.writeUInt16LE(se - base[0], i * 7);
    body.writeUInt16LE(sn - base[1], i * 7 + 2);
    body.writeUInt16LE(sg - base[2], i * 7 + 4);
    body.writeUInt8(flag, i * 7 + 6);
  });
  return Buffer.concat([header, body]);
}

export function build(sourceDirectory) {
  const data = readFileSync(join(sourceDirectory, source.data.name));
  check(data, source.data, 'OS OSTN15/OSGM15 data file');
  const output = pack(parseDataFile(data.toString('latin1')));
  check(output, source.output, 'OSTN15 package');
  const path = join(root, source.output.path);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, output);
  return output;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  if (process.argv.length !== 3) throw new Error('usage: ostn15-assets.mjs <source-directory>');
  const out = build(process.argv[2]);
  console.log(`OSTN15 asset: ${out.length} bytes, sha256 ${sha256(out)}`);
}
