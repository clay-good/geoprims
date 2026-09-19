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
