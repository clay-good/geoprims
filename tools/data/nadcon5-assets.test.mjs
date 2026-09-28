import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { pack, parseNgsGrid, sources } from './nadcon5-assets.mjs';

const root = new URL('../..', import.meta.url).pathname;
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

function ngsGrid({ south = 1, west = 2, latStep = 0.5, lonStep = 0.25, rows = 3, cols = 3, start = 0 }) {
  const rowBytes = cols * 4;
  const bytes = Buffer.alloc(52 + rows * (rowBytes + 8));
  bytes.writeInt32BE(44, 0);
  bytes.writeDoubleBE(south, 4);
  bytes.writeDoubleBE(west, 12);
  bytes.writeDoubleBE(latStep, 20);
  bytes.writeDoubleBE(lonStep, 28);
  bytes.writeInt32BE(rows, 36);
  bytes.writeInt32BE(cols, 40);
  bytes.writeInt32BE(1, 44);
  bytes.writeInt32BE(44, 48);
  let offset = 52;
  for (let row = 0; row < rows; row++) {
    bytes.writeInt32BE(rowBytes, offset);
    offset += 4;
    for (let col = 0; col < cols; col++) bytes.writeFloatBE(start + row * cols + col, offset + col * 4);
    offset += rowBytes;
    bytes.writeInt32BE(rowBytes, offset);
    offset += 4;
  }
  return bytes;
}

test('the committed NADCON5 packages have the pinned output digests', () => {
  assert.equal(sources.length, 7);
  for (const source of sources) {
    assert.match(source.lat.url, /^https:\/\/geodesy\.noaa\.gov\/pub\/nadcon5\/20160901release\/Builds\//);
    assert.match(source.proj.url, /^https:\/\/cdn\.proj\.org\/us_noaa_nadcon5_/);
    const bytes = readFileSync(join(root, source.output.path));
    assert.equal(bytes.length, source.output.bytes, `${source.outputName} bytes`);
    assert.equal(sha256(bytes), source.output.sha256, `${source.outputName} sha256`);
  }
});

test('the packer reads NGS Fortran records and writes interleaved little-endian samples', () => {
  const lat = ngsGrid({ west: 235, start: 1 });
  const lon = ngsGrid({ west: 235, start: -9 });
  const parsed = parseNgsGrid(lat);
  assert.deepEqual(parsed.samples, [1, 2, 3, 4, 5, 6, 7, 8, 9]);
  const output = pack(lat, lon);
  const newline = output.indexOf(10);
  assert.equal(output.subarray(0, newline).toString(), 'NADCON5 -125.0 1.0 0.25 0.5 3 3');
  assert.equal(output.readFloatLE(newline + 1), 1);
  assert.equal(output.readFloatLE(newline + 5), -9);
  assert.equal(output.readFloatLE(output.length - 8), 9);
  assert.equal(output.readFloatLE(output.length - 4), -1);
});

test('the NGS parser rejects a damaged row record', () => {
  const bytes = ngsGrid({});
  bytes.writeInt32BE(8, 52);
  assert.throws(() => parseNgsGrid(bytes), /row 1/);
});
