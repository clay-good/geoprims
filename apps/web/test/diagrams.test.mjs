// Vector diagrams (web/map-canvas "Canvas modes"): each diagram tool draws an
// SVG from its worked example's real result, described in words, with no
// inline styles (the CSP allows none) and no color literals.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { nodeHost } from '../../../packages/runtime/src/node.mjs';
import { DIAGRAM_TOOLS, diagram, measure } from '../src/lib/diagrams.js';

const web = new URL('..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(web, '../../dist/catalog/v1.json'), 'utf8'));
const host = nodeHost(join(web, '../../dist/wasm'));

test('every diagram tool draws its worked example', async () => {
  for (const id of DIAGRAM_TOOLS) {
    const t = catalog.tools.find((x) => x.id === id);
    assert.ok(t, id);
    const args = (t.examples.find((e) => e.id === t['x-primary-example']) ?? t.examples[0]).input;
    const result = JSON.parse(await host.invoke(id, JSON.stringify(args)));
    const d = diagram(id, args, result);
    assert.ok(d, `${id}: no diagram`);
    assert.match(d.markup, /^<svg viewBox="0 0 320 240"/);
    assert.doesNotMatch(d.markup, /style=|#[0-9a-f]{3,6}\b|NaN|undefined/i, id);
    assert.ok(d.desc.length > 20, id);
  }
});

test('the wind triangle is described with the core values', async () => {
  const args = { course: '90 deg', tas: '120 kt', wind_direction: '30 deg', wind_speed: '20 kt' };
  const r = JSON.parse(await host.invoke('aviation.wind.heading-groundspeed', JSON.stringify(args)));
  const d = diagram('aviation.wind.heading-groundspeed', args, r);
  assert.ok(d.desc.includes(r.display.heading) && d.desc.includes(r.display.groundspeed), d.desc);
  assert.equal(diagram('aviation.wind.heading-groundspeed', args, { ok: false }), null);
  assert.equal(diagram('units.speed.convert', args, r), null);
});

test('speeds and lengths normalize across units', () => {
  const kt = 1852 / 3600;
  assert.equal(measure('120 kt', { kt, 'm/s': 1 }, 'kt'), 120 * kt);
  assert.equal(measure('10 m/s', { kt, 'm/s': 1 }, 'kt'), 10);
  assert.equal(measure(5, { kt, 'm/s': 1 }, 'kt'), 5 * kt);
  assert.equal(measure('fast', { kt }, 'kt'), null);
});

test('the CPA scene draws both positions at the playhead from the core', async () => {
  const args = { a_course: '090 deg', a_speed: '10 m/s', b_east: '1000 m', b_north: '1200 m', b_course: '180 deg', b_speed: '10 m/s', at_time: '60 s' };
  const r = JSON.parse(await host.invoke('navigation.route.cpa', JSON.stringify(args)));
  const d = diagram('navigation.route.cpa', args, r);
  assert.equal((d.markup.match(/class="dg-dot-now"/g) ?? []).length, 2);
  assert.ok(d.markup.includes(`t = 60 s · ${r.display.separation_at} apart`), d.markup);
  const cpa = catalog.tools.find((t) => t.id === 'navigation.route.cpa');
  assert.deepEqual(cpa.timeline, { input: 'at_time', end: 'scene_end', key: 'time' });
});

test('the sky plot shows the target by azimuth and elevation', async () => {
  // reference-frames "Sky plot".
  const { nodeHost } = await import('../../../packages/runtime/src/node.mjs');
  const host = nodeHost(new URL('../../../dist/wasm', import.meta.url).pathname);
  const args = { lat0: 40, lon0: -105, h0: 1600, lat: 40.05, lon: -104.95, height: 3000 };
  const r = JSON.parse(await host.invoke('geodesy.frame.to-local', JSON.stringify(args)));
  const d = diagram('geodesy.frame.to-local', args, r);
  assert.match(d.desc, new RegExp(`azimuth ${r.display.azimuth.replace('.', '\\.')} and elevation`));
  // The dot is north-east of the center, and nearer the rim than the zenith at a low elevation.
  const [, x, y] = /<circle class="dg-dot" cx="([\d.]+)" cy="([\d.]+)"/.exec(d.markup).map(Number);
  assert.ok(x > 160 && y < 122, `${x}, ${y}`);
  assert.ok(Math.hypot(x - 160, y - 122) > 48);
  const under = JSON.parse(await host.invoke('geodesy.frame.to-local', JSON.stringify({ ...args, height: -500, lat: 45 })));
  assert.match(diagram('geodesy.frame.to-local', args, under).desc, /below the horizon/);
});

test('profiles, the traverse sketch, and the airspeed dial draw from core values', async () => {
  const { join } = await import('node:path');
  const { nodeHost } = await import('../../../packages/runtime/src/node.mjs');
  const { station } = await import('../src/lib/diagrams.js');
  const root = join(new URL('..', import.meta.url).pathname, '../..');
  const host = nodeHost(join(root, 'dist/wasm'));
  const run = async (id, args) => [args, JSON.parse(await host.invoke(id, JSON.stringify(args)))];

  // Vertical curve: the drawn parabola ends where the core puts PVC and PVT.
  const [va, vr] = await run('survey.curves.vertical-curve', { g1: 2, g2: -3, length: '600 ft', pvi_elevation: '100 ft', pvi_station: '10+00' });
  const L = station(vr.result.pvt_station) - station(vr.result.pvc_station);
  const yEnd = vr.result.pvc_elevation.value + (va.g1 / 100) * L + ((va.g2 - va.g1) / (200 * L)) * L * L;
  assert.ok(Math.abs(yEnd - vr.result.pvt_elevation.value) < 1e-9, 'the curve reaches the PVT elevation');
  const vc = diagram('survey.curves.vertical-curve', va, vr);
  assert.match(vc.markup, /PVC 7\+00\.00/);
  assert.match(vc.desc, /high point at 9\+40\.00/);
  assert.equal(station('7+00.00'), 700);
  assert.equal(station('12+34.5'), 1234.5);

  // Descent and approach profiles name the core's distance and rate.
  const [da, dr] = await run('aviation.performance.top-of-descent', { from_altitude: '35000 ft', to_altitude: '3000 ft', groundspeed: '420 kt' });
  assert.ok(diagram('aviation.performance.top-of-descent', da, dr).markup.includes(dr.display.distance));
  const [pa, pr] = await run('aviation.performance.vdp', { height_above_touchdown: '400 ft', groundspeed: '90 kt' });
  assert.match(diagram('aviation.performance.vdp', pa, pr).desc, new RegExp(pr.display.distance));

  // Traverse: one numbered dot per course and the misclosure named.
  const calls = [{ direction: "N 0°00'00\" E", distance: '500 ftUS' }, { direction: "N 90°00'00\" E", distance: '425 ftUS' }, { direction: "S 0°00'00\" E", distance: '500 ftUS' }, { direction: "S 89°56'36\" W", distance: '425 ftUS' }];
  const [ta, tr] = await run('survey.land.deed-plot', { calls });
  const ts = diagram('survey.land.deed-plot', ta, tr);
  assert.equal((ts.markup.match(/class="dg-dot(-now)?"/g) ?? []).length, 4);
  assert.match(ts.desc, /4 courses/);

  // Airspeed dial: TAS sits clockwise of CAS when TAS is the larger.
  const [aa, ar] = await run('aviation.airspeed.cas-to-tas', { airspeed: '250 kt', pressure_altitude: '10000 ft', temperature: '-5 degC' });
  const g = diagram('aviation.airspeed.cas-to-tas', aa, ar);
  assert.match(g.desc, /true airspeed 288\.6/);
  assert.equal(diagram('aviation.airspeed.cas-to-tas', aa, { ok: true, result: { cas: { value: 1, unit: 'kt' } } }), null);
});

test('schematics say so, and units with digits parse (ft2)', async () => {
  const { DIAGRAM_TOOLS: all } = await import('../src/lib/diagrams.js');
  for (const id of ['aviation.atmosphere.isa', 'aviation.performance.climb-gradient', 'navigation.los.horizon', 'navigation.los.visibility', 'navigation.los.fresnel', 'survey.earthwork.average-end-area', 'survey.earthwork.prismoidal']) {
    assert.ok(all.includes(id), `${id}: no diagram`);
  }
  assert.equal(measure('120 ft2', { ft2: 2 }, 'ft2'), 240);
  assert.equal(measure('10 m/s', { 'm/s': 1 }, 'm/s'), 10);
  assert.equal(measure('12', { kt: 3 }, 'kt'), 36);
});
