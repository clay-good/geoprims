import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { source } from './beta2007-assets.mjs';

const root = new URL('../..', import.meta.url).pathname;
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

test('the committed BeTA2007 grid has the pinned digest and AdV\'s extent', () => {
  assert.match(source.url, /^https:\/\/cdn\.proj\.org\/de_adv_BETA2007\.tif$/);
  const bytes = readFileSync(join(root, source.output.path));
  assert.equal(bytes.length, source.output.bytes);
  assert.equal(sha256(bytes), source.output.sha256);
  const header = bytes.subarray(0, bytes.indexOf(10)).toString();
  // Nodes every 10 arcminutes of longitude and 6 of latitude from 5°30'E
  // 47°N, 62 by 84: AdV's extent, 5°30'E to 15°40'E and 47°N to 55°18'N.
  assert.equal(header, 'HGRID 5.5 47 0.1666666666666667 0.1 62 84');
  assert.equal(bytes.length, header.length + 1 + 62 * 84 * 8);
  const [, west, south, dlon, dlat] = header.split(' ').map(Number);
  assert.ok(Math.abs(dlon - 10 / 60) < 1e-16 && Math.abs(dlat - 6 / 60) < 1e-16);
  assert.ok(Math.abs(west + 61 * dlon - (15 + 40 / 60)) < 1e-6, 'east edge 15°40′E');
  assert.ok(Math.abs(south + 83 * dlat - (55 + 18 / 60)) < 1e-6, 'north edge 55°18′N');
});
