import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { source } from './chenyx06-assets.mjs';

const root = new URL('../..', import.meta.url).pathname;
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

test('the committed CHENyx06 grid has the pinned digest and swisstopo\'s extent', () => {
  assert.match(source.url, /^https:\/\/cdn\.proj\.org\/ch_swisstopo_CHENyx06a\.tif$/);
  const bytes = readFileSync(join(root, source.output.path));
  assert.equal(bytes.length, source.output.bytes);
  assert.equal(sha256(bytes), source.output.sha256);
  const header = bytes.subarray(0, bytes.indexOf(10)).toString();
  // Nodes every 30 arcseconds from 5°33'E 45°28'N, 661 by 313: swisstopo's
  // extent, 5°33'E to 11°03'E and 45°28'N to 48°04'N. The spacing is written
  // at full precision, because 30 arcseconds is not a round decimal: rounded
  // to nine decimals it moves the interpolated offsets by up to 1.1 um.
  assert.equal(header, 'HGRID 5.55 45.466666666667 0.0083333333333333 0.0083333333333333 661 313');
  assert.equal(bytes.length, header.length + 1 + 661 * 313 * 8);
  const [, west, south, dlon, dlat] = header.split(' ').map(Number);
  assert.ok(Math.abs(dlon - 1 / 120) < 1e-16 && Math.abs(dlat - 1 / 120) < 1e-16);
  assert.ok(Math.abs(west + 660 * dlon - (11 + 3 / 60)) < 1e-6, 'east edge 11°03′E');
  assert.ok(Math.abs(south + 312 * dlat - (48 + 4 / 60)) < 1e-6, 'north edge 48°04′N');
});
