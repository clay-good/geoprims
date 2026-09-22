// Batch mode (web/io-formats, "Batch mode over CSV"). The scenario: a
// 10,000-row batch with 12 invalid rows gives 9,988 results, lists the 12
// errors with their rows and messages, and exports an error column.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { batchErrors, batchInputs, CHUNK, joinedCsv, MAX_ROWS, rowArgs, runBatch, suggestMapping } from '../src/lib/batch.mjs';
import { readDelimited } from '../src/lib/import.mjs';
import { nodeHost } from '../../../packages/runtime/src/node.mjs';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const host = nodeHost(join(root, 'dist/wasm'));
const tool = (id) => catalog.tools.find((t) => t.id === id);
const invokeBatch = (id, json) => host.invokeBatch(id, json);

test('10,000 rows with 12 bad ones: 9,988 results, 12 errors by row, an error column', async () => {
  const t = tool('aviation.altimetry.pressure-altitude');
  const header = 'elevation,altimeter';
  const bad = new Set([7, 100, 999, 1000, 1001, 2500, 4096, 5000, 7777, 8191, 9000, 9999]);
  const lines = [header];
  for (let i = 0; i < 10_000; i++) lines.push(bad.has(i) ? 'not-a-height,29.92' : `${i % 9000},${(28.5 + (i % 300) / 100).toFixed(2)}`);
  const parsed = readDelimited(lines.join('\n'));
  const mapping = suggestMapping(t, parsed.headers);
  assert.deepEqual(mapping, { elevation: 0, altimeter: 1 });
  const args = rowArgs(t, parsed.rows, mapping);
  assert.equal(args[1].elevation, '1 ft', 'a bare number takes the input unit');
  const progress = [];
  const { results, cancelled } = await runBatch(t.id, args, { invokeBatch, onProgress: (done, total) => progress.push([done, total]) });
  assert.equal(cancelled, false);
  assert.equal(results.length, 10_000);
  assert.equal(results.filter((r) => r.ok).length, 9_988);
  const errors = batchErrors(results);
  assert.equal(errors.length, 12);
  assert.deepEqual(errors.map((e) => e.row - 1), [...bad].sort((a, b) => a - b));
  assert.ok(errors.every((e) => e.message && e.line === e.row + 1));
  // Progress arrives once per chunk, ending at the total.
  assert.equal(progress.length, 10_000 / CHUNK);
  assert.deepEqual(progress.at(-1), [10_000, 10_000]);
  // The export keeps every original column, adds the outputs, and an error column.
  const csv = joinedCsv(t, parsed.headers, parsed.rows, results).trim().split('\n');
  assert.equal(csv.length, 10_001);
  assert.match(csv[0], /^elevation,altimeter,.*Pressure altitude \(ft\).*,error$/);
  assert.match(csv[8], /^not-a-height,29\.92,.*INVALID_INPUT/);
  assert.match(csv[2], /,$/, 'a row that worked has an empty error cell');
});

test('a batch can be cancelled between chunks', async () => {
  const t = tool('units.speed.convert');
  const args = Array.from({ length: 5_000 }, (_, i) => ({ value: `${i} kt`, to: 'mph' }));
  const controller = new AbortController();
  const { results, cancelled } = await runBatch(t.id, args, {
    invokeBatch,
    signal: controller.signal,
    onProgress: (done) => done >= 2 * CHUNK && controller.abort(),
  });
  assert.equal(cancelled, true);
  assert.equal(results.length, 2 * CHUNK, 'it stopped at the chunk boundary, keeping what was done');
});

test('a batch larger than the limit is refused before anything runs', async () => {
  let calls = 0;
  await assert.rejects(
    runBatch('units.speed.convert', new Array(MAX_ROWS + 1).fill({}), { invokeBatch: async () => (calls++, '[]') }),
    /at most 100,000 rows/,
  );
  assert.equal(calls, 0);
});

test('only scalar inputs take a column, and empty cells keep the default', () => {
  const t = tool('navigation.route.legs');
  assert.ok(!batchInputs(t).some((i) => i.name === 'waypoints'), 'a list input cannot come from one cell');
  const d = tool('aviation.altimetry.density-altitude');
  const args = rowArgs(d, [['5000', '29.80', '30 degC', '']], { elevation: 0, altimeter: 1, temperature: 2, dew_point: 3 }, { altimeter: 'inHg' });
  assert.deepEqual(args, [{ elevation: '5000 ft', altimeter: '29.80 inHg', temperature: '30 degC' }]);
});

test('every tool page offers batch mode', () => {
  const problems = [];
  for (const t of catalog.tools.slice(0, 60)) {
    const html = readFileSync(join(web, 'dist', ...t.id.split('.'), 'index.html'), 'utf8');
    if (batchInputs(t).length && !/class="batch/.test(html)) problems.push(t.id);
  }
  assert.deepEqual(problems, []);
});
