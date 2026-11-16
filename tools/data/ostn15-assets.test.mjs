import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { COLS, ROWS, pack, parseDataFile, source } from './ostn15-assets.mjs';

const root = new URL('../..', import.meta.url).pathname;
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

test('the committed OSTN15 package has the pinned digest and OS as its source', () => {
  assert.match(source.zip.url, /^https:\/\/www\.ordnancesurvey\.co\.uk\/documents\/resources\//);
  const bytes = readFileSync(join(root, source.output.path));
  assert.equal(bytes.length, source.output.bytes);
  assert.equal(sha256(bytes), source.output.sha256);
  const header = bytes.subarray(0, bytes.indexOf(10)).toString();
  assert.match(header, new RegExp(`^OSTN15 ${COLS} ${ROWS} -?\\d+ -?\\d+ -?\\d+$`));
  assert.equal(bytes.length, header.length + 1 + COLS * ROWS * 7);
});

test('the packer keeps every value to the millimetre and checks the record order', () => {
  const lines = ['Point_ID,ETRS89_Easting,ETRS89_Northing,ETRS89_OSGB36_EShift,ETRS89_OSGB36_NShift,ETRS89_ODN_HeightShift,Height_Datum_Flag'];
  for (let r = 0; r < COLS * ROWS; r++) {
    const e = (r % COLS) * 1000;
    const n = Math.floor(r / COLS) * 1000;
    lines.push(`${r + 1},${e},${n},${(90 + (r % 7) * 0.001).toFixed(3)},${(-80 - (r % 5) * 0.001).toFixed(3)},${(50.25).toFixed(3)},${r === 0 ? 2 : 1}`);
  }
  const nodes = parseDataFile(lines.join('\n'));
  assert.deepEqual(nodes[0], [90000, -80000, 50250, 2]);
  assert.deepEqual(nodes[6], [90006, -80001, 50250, 1]);
  const bytes = pack(nodes);
  const nl = bytes.indexOf(10);
  assert.equal(bytes.subarray(0, nl).toString(), `OSTN15 ${COLS} ${ROWS} 90000 -80004 50250`);
  const at = (i, k) => bytes.readUInt16LE(nl + 1 + i * 7 + k * 2);
  assert.equal(at(6, 0), 6);
  assert.equal(at(4, 1), 0);
  assert.equal(bytes.readUInt8(nl + 1 + 6), 2);
  // A record out of place is refused rather than packed.
  const swapped = [...lines];
  [swapped[2], swapped[3]] = [swapped[3], swapped[2]];
  assert.throws(() => parseDataFile(swapped.join('\n')), /out of order/);
});
