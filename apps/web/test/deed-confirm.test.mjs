import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import { nodeHost } from '../../../packages/runtime/src/node.mjs';
import { deedPlotState, plotCalls, sourceParts } from '../src/lib/deed-confirm.mjs';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const page = readFileSync(join(web, 'dist/survey/land/deed-parse/index.html'), 'utf8');
const host = nodeHost(join(root, 'dist/wasm'));

const text = 'Beginning; thence N 10° E 50 feet; thence along the creek; thence along a curve to the right.';
const rows = [
  { kind: 'line', direction: 'N 10°00\'00" E', distance: '50 ftUS', source: 'thence N 10° E 50 feet' },
  { kind: 'non-metric', source: 'thence along the creek', flags: 'NON_METRIC_CALL' },
  { kind: 'curve', chord_direction: 'N 80 E', chord: '40 ftUS', radius: '100 ftUS', turn: 'right', source: 'thence along a curve to the right' },
];

test('every parsed source phrase is linked back to the original deed text', () => {
  const parts = sourceParts(text, rows);
  assert.equal(parts.map((part) => part.text).join(''), text);
  assert.deepEqual(parts.filter((part) => part.call !== null).map((part) => part.call), [0, 1, 2]);
});

test('repeated source phrases link to their successive calls', () => {
  const repeated = 'thence north 10 feet; thence north 10 feet';
  const parts = sourceParts(repeated, [{ source: 'thence north 10 feet' }, { source: 'thence north 10 feet' }]);
  assert.deepEqual(parts.filter((part) => part.call !== null).map((part) => part.call), [0, 1]);
  assert.equal(parts.map((part) => part.text).join(''), repeated);
});

test('only confirmed geometric fields reach the deed plot', () => {
  assert.deepEqual(plotCalls(rows), [
    { kind: 'line', direction: 'N 10°00\'00" E', distance: '50 ftUS' },
    { kind: 'curve', chord_direction: 'N 80 E', chord: '40 ftUS', radius: '100 ftUS', turn: 'right' },
  ]);
  assert.deepEqual(deedPlotState(rows), {
    i: { calls: plotCalls(rows) },
    c: 'survey.land.deed-parse',
  });
});

test('confirmed rows survive the link handoff and run in the deed plotter', async () => {
  const confirmed = [
    { kind: 'line', direction: 'N 0 E', distance: '100 ftUS' },
    { kind: 'line', direction: 'N 90 E', distance: '100 ftUS' },
    { kind: 'line', direction: 'S 0 E', distance: '100 ftUS' },
    { kind: 'line', direction: 'S 90 W', distance: '100 ftUS' },
  ];
  const state = deedPlotState(confirmed);
  const link = await host.module('link');
  const encoded = JSON.parse(await link.callString('gp_link_encode', JSON.stringify({ state })));
  assert.ok(encoded.ok, JSON.stringify(encoded));
  const decoded = JSON.parse(await link.callString('gp_link_decode', encoded.result.fragment));
  assert.deepEqual(decoded.result.state, state);

  const plotted = JSON.parse(await host.invoke('survey.land.deed-plot', JSON.stringify(decoded.result.state.i)));
  assert.ok(plotted.ok, JSON.stringify(plotted));
  assert.equal(plotted.result.points.length, 5);
  assert.ok(plotted.result.misclosure.value < 1e-9);
});

test('the deed page renders the editable confirmation workflow before hydration', () => {
  assert.match(page, /Confirm the parsed calls/);
  assert.match(page, /Parsed calls — editable until you confirm/);
  assert.match(page, /Confirm calls and open deed plot/);
  assert.match(page, /Original deed with one highlighted phrase per parsed call/);
});
