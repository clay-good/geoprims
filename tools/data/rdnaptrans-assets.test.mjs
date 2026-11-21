import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { sources } from './rdnaptrans-assets.mjs';

const root = new URL('../..', import.meta.url).pathname;
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');
const read = (name) => readFileSync(join(root, 'assets/data/rdnaptrans/2018', name));
const header = (bytes, from = 0) => bytes.subarray(from, bytes.indexOf(10, from)).toString();

test('the committed RDNAPTRANS2018 grids have the pinned digests', () => {
  for (const source of sources) {
    assert.match(source.url, /^https:\/\/cdn\.proj\.org\/nl_nsgi_\w+\.tif$/);
    const bytes = readFileSync(join(root, source.output.path));
    assert.equal(bytes.length, source.output.bytes);
    assert.equal(sha256(bytes), source.output.sha256);
  }
});

test('the correction grid is NSGI\'s parent and subgrid, at their published bounds', () => {
  const bytes = read('rdtrans2018.grid');
  assert.equal(header(bytes), 'HGRIDS 2');
  // Parent NL_EEZ: 50°N to 56°N and 2°E to 8°E at 0.1°, so 61 by 61.
  const parent = header(bytes, 9);
  assert.equal(parent, 'HGRID 2 50 0.1 0.1 61 61');
  const afterParent = 9 + parent.length + 1 + 61 * 61 * 8;
  // Subgrid NL: 50°N to 54°N and 2.5°E to 8°E at 0.02° by 0.0125°, so 276 by 321.
  const sub = header(bytes, afterParent);
  assert.equal(sub, 'HGRID 2.5 50 0.02 0.0125 276 321');
  assert.equal(bytes.length, afterParent + sub.length + 1 + 276 * 321 * 8);
});

test('the quasi-geoid is NLGEO2018 at its published bounds and spacing', () => {
  const bytes = read('nlgeo2018.grid');
  const head = header(bytes);
  // 50°N to 56°N and 2°E to 8°E at 0.0125° by 0.02°, so 301 by 481.
  assert.equal(head, 'VGRID 2 50 0.02 0.0125 301 481');
  assert.equal(bytes.length, head.length + 1 + 301 * 481 * 4);
  // Every node is a quasi-geoid height over the North Sea and the Netherlands,
  // which NSGI's grid puts between 39 and 49 m.
  const body = bytes.subarray(head.length + 1);
  let min = Infinity;
  let max = -Infinity;
  for (let i = 0; i < body.length; i += 4) {
    const v = body.readFloatLE(i);
    if (v < min) min = v;
    if (v > max) max = v;
  }
  assert.ok(min > 39 && min < 40, `minimum ${min}`);
  assert.ok(max > 48 && max < 49, `maximum ${max}`);
});
