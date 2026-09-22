// The map's cursor readout (web/map-canvas, "Measurement readouts"): the point
// under the pointer in the reader's coordinate format, grid formats from the
// core's own tools, and the magnetic north indicator from the result.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { formatDegrees, magneticNorth, mgrsPrecision, readoutText, wrapLon } from '../src/lib/map/readout.js';
import { COORD_FORMATS } from '../src/lib/prefs.js';
import { nodeHost } from '../../../packages/runtime/src/node.mjs';

const web = new URL('..', import.meta.url).pathname;
const host = nodeHost(join(web, '../../dist/wasm'));
const invoke = async (id, input) => JSON.parse(await host.invoke(id, JSON.stringify(input)));

test('cursor readout format: with MGRS chosen, the readout is the MGRS of the point', async () => {
  const text = await readoutText(40.446111, -79.982222, 'mgrs', { invoke, metersPerPixel: 0.5 });
  assert.equal(text, '17T NE 86309 77770', 'the same reference the MGRS tool gives');
  // Zoomed out, the reference names a square about one pixel across, not a meter.
  assert.equal(await readoutText(40.446111, -79.982222, 'mgrs', { invoke, metersPerPixel: 800 }), '17T NE 86 77', 'a 1 km square at 800 m a pixel');
  // Near the poles MGRS switches to UPS; the core says so, and the readout follows it.
  assert.match(await readoutText(85, 10, 'mgrs', { invoke, metersPerPixel: 0.5 }), /^Z AB /);
});

test('UTM reads from the core, and outside the grid the readout falls back to degrees and says so', async () => {
  assert.equal(await readoutText(40.446111, -79.982222, 'utm', { invoke }), '17N 586310 mE 4477770 mN');
  assert.equal(await readoutText(86, 10, 'utm', { invoke }), '86.0000°, 10.0000° (outside UTM)');
  // A pointer over a panned map may sit past the antimeridian.
  assert.equal(await readoutText(40.446111, 280.017778, 'utm', { invoke }), '17N 586310 mE 4477770 mN');
});

test('degree formats round once, at the last digit shown', () => {
  assert.equal(formatDegrees(40.446111, -79.982222), '40.4461°, -79.9822°');
  assert.equal(formatDegrees(40.446111, -79.982222, 'dms'), '40° 26′ 46.0″ N, 79° 58′ 56.0″ W');
  assert.equal(formatDegrees(40.446111, -79.982222, 'ddm'), '40° 26.767′ N, 79° 58.933′ W');
  // 59.99999″ must carry into the minute, never print as 60.0″.
  assert.equal(formatDegrees(10 + 59 / 60 + 59.99999 / 3600, 0, 'dms'), '11° 00′ 00.0″ N, 0° 00′ 00.0″ E');
  assert.equal(formatDegrees(-0.5, 0.5, 'ddm'), '0° 30.000′ S, 0° 30.000′ E');
});

test('the MGRS precision resolves about one pixel, and longitudes fold into range', () => {
  assert.equal(mgrsPrecision(0.4), '1m');
  assert.equal(mgrsPrecision(7), '10m');
  assert.equal(mgrsPrecision(4000), '10km');
  assert.equal(mgrsPrecision(90000), '100km');
  assert.equal(wrapLon(190), -170);
  assert.equal(wrapLon(-190), 170);
  assert.equal(wrapLon(-80), -80);
});

test('every coordinate format the settings offer is one the readout writes', async () => {
  for (const [id] of COORD_FORMATS) {
    const text = await readoutText(40.446111, -79.982222, id, { invoke, metersPerPixel: 1 });
    assert.ok(text && !/NaN|undefined/.test(text), `${id}: ${text}`);
  }
});

test('magnetic north shows only when the result carries a declination', async () => {
  const catalog = JSON.parse(readFileSync(join(web, '../../dist/catalog/v1.json'), 'utf8'));
  const tool = catalog.tools.find((t) => t.id === 'geodesy.magnetic.declination');
  const args = (tool.examples.find((e) => e.id === tool['x-primary-example']) ?? tool.examples[0]).input;
  const result = await invoke(tool.id, args);
  if (result.ok) assert.equal(magneticNorth(result), result.result.declination.value);
  assert.equal(magneticNorth({ ok: true, result: { declination: { value: -9.5, unit: 'deg' } } }), -9.5);
  assert.equal(magneticNorth({ ok: true, result: { distance: { value: 1, unit: 'm' } } }), null);
  assert.equal(magneticNorth({ ok: false, error: {} }), null);
});
