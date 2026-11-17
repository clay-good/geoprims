import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { source } from './nzgd2000-grid-assets.mjs';

const root = new URL('../..', import.meta.url).pathname;
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

test('the committed NZGD2000 grid has the pinned digest and the grid LINZ publishes', () => {
  assert.match(source.url, /^https:\/\/cdn\.proj\.org\/nz_linz_nzgd2kgrid0005\.tif$/);
  const bytes = readFileSync(join(root, source.output.path));
  assert.equal(bytes.length, source.output.bytes);
  assert.equal(sha256(bytes), source.output.sha256);
  const header = bytes.subarray(0, bytes.indexOf(10)).toString();
  // Nodes every 0.1° from 166°E and 48°S, 141 by 141: LINZ's extent, 166°E to
  // 180°E and 34°S to 48°S.
  assert.equal(header, 'HGRID 166 -48 0.1 0.1 141 141');
  assert.equal(bytes.length, header.length + 1 + 141 * 141 * 8);
});
