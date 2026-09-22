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

/** Every `<line>` in a drawing, as {cls, x1, y1, x2, y2}. */
const lines = (markup) =>
  [...markup.matchAll(/<line class="([^"]*)" x1="([-\d.]+)" y1="([-\d.]+)" x2="([-\d.]+)" y2="([-\d.]+)"/g)].map((m) => ({
    cls: m[1],
    x1: Number(m[2]),
    y1: Number(m[3]),
    x2: Number(m[4]),
    y2: Number(m[5]),
  }));
/** A screen vector's compass bearing (y grows downward) and length. */
const bearing = (l) => ({
  deg: ((Math.atan2(l.x2 - l.x1, l.y1 - l.y2) * 180) / Math.PI + 360) % 360,
  len: Math.hypot(l.x2 - l.x1, l.y1 - l.y2),
});
const near = (a, b, tol, what) => assert.ok(Math.abs(((a - b + 540) % 360) - 180) < tol, `${what}: ${a} vs ${b}`);

test('the wind triangle closes: air vector, then wind, reach the ground vector', async () => {
  // add-aviation-suite 4.6 fixture.
  for (const args of [
    { course: '90 deg', tas: '120 kt', wind_direction: '30 deg', wind_speed: '20 kt' },
    { course: '350 deg', tas: '95 kt', wind_direction: '260 deg', wind_speed: '35 kt' },
    { course: '180 deg', tas: '140 kt', wind_direction: '180 deg', wind_speed: '15 kt' },
  ]) {
    const r = JSON.parse(await host.invoke('aviation.wind.heading-groundspeed', JSON.stringify(args)));
    const d = diagram('aviation.wind.heading-groundspeed', args, r);
    const [air, wind, ground] = lines(d.markup).filter((l) => l.cls !== 'dg-casing');
    // The air vector is drawn at the heading the core computed, the ground
    // vector along the requested course, and the three meet head to tail.
    near(bearing(air).deg, r.result.heading.value, 0.5, 'air vector');
    near(bearing(ground).deg, Number.parseFloat(args.course), 0.5, 'ground vector');
    near(bearing(wind).deg, (Number.parseFloat(args.wind_direction) + 180) % 360, 0.5, 'wind vector');
    assert.ok(Math.hypot(air.x2 - wind.x1, air.y2 - wind.y1) < 0.2, 'wind starts where the air vector ends');
    assert.ok(Math.hypot(wind.x2 - ground.x2, wind.y2 - ground.y2) < 0.2, 'the wind reaches the ground vector');
    // Lengths are to scale: ground / air = groundspeed / true airspeed.
    const ratio = r.result.groundspeed.value / Number.parseFloat(args.tas);
    assert.ok(Math.abs(bearing(ground).len / bearing(air).len - ratio) < 0.02, 'to scale');
  }
});

test('the runway drawing puts the wind components along and across the runway', async () => {
  for (const args of [
    { runway: '09', wind_direction: '120 deg', wind_speed: '15 kt' },
    { runway: '09', wind_direction: '060 deg', wind_speed: '15 kt' },
    { runway: '36', wind_direction: '170 deg', wind_speed: '20 kt' },
  ]) {
    const r = JSON.parse(await host.invoke('aviation.wind.runway-components', JSON.stringify(args)));
    const d = diagram('aviation.wind.runway-components', args, r);
    const drawn = lines(d.markup).filter((l) => l.cls !== 'dg-casing');
    const runway = drawn.find((l) => l.cls.includes('dg-runway'));
    const [head, cross] = drawn.filter((l) => l.cls.includes('dg-accent'));
    const rwy = r.result.runway_heading.value;
    near(bearing(runway).deg, rwy, 0.5, 'runway');
    // A headwind arrow points back down the runway, a tailwind along it.
    const hw = r.result.headwind.value;
    near(bearing(head).deg, rwy + (hw >= 0 ? 180 : 0), 0.5, 'headwind arrow');
    // The crosswind arrow is square to the runway, pointing the way the wind
    // blows: to the right of the runway when the wind comes from its left.
    const blows = (Number.parseFloat(args.wind_direction) + 180 - rwy + 720) % 360;
    const across = (bearing(cross).deg - rwy + 720) % 360;
    assert.ok(Math.abs(across - (blows < 180 ? 90 : 270)) < 0.5, `crosswind drawn ${across} for wind blowing ${blows}`);
    // Both are to the same scale as the wind they came from.
    const k = bearing(head).len / Math.abs(hw);
    assert.ok(Math.abs(bearing(cross).len / Math.abs(r.result.crosswind.value) - k) < 0.02, 'components to one scale');
    assert.match(d.markup, /Crosswind/);
  }
});

/** Even-odd membership, the same rule the core uses for the envelope. */
const inside = (ring, [x, y]) => {
  let on = false;
  for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
    const [[xi, yi], [xj, yj]] = [ring[i], ring[j]];
    if (yi > y !== yj > y && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) on = !on;
  }
  return on;
};
const circles = (markup) =>
  [...markup.matchAll(/<circle class="([^"]*)" cx="([-\d.]+)" cy="([-\d.]+)"/g)].map((m) => ({ cls: m[1], x: Number(m[2]), y: Number(m[3]) }));

test('the weight-and-balance chart puts the loaded point where the core says it is', async () => {
  // add-aviation-suite 6.3 fixture.
  const stations = [
    { name: 'Empty', weight: '1500 lb', arm: '85 in' },
    { name: 'Front seats', weight: '340 lb', arm: '90 in' },
    { name: 'Rear seats', weight: '170 lb', arm: '118 in' },
    { name: 'Fuel', weight: '240 lb', arm: '48 in' },
  ];
  const envelope = [
    { arm: '82 in', weight: '1500 lb' },
    { arm: '93 in', weight: '1500 lb' },
    { arm: '93 in', weight: '2300 lb' },
    { arm: '84 in', weight: '2300 lb' },
    { arm: '82 in', weight: '1950 lb' },
  ];
  // The second load is nose-heavy enough to fall out the front of the envelope.
  for (const args of [
    { stations, fuel_burn: '60 lb', envelope },
    { stations: [...stations.slice(0, 1), { name: 'Nose ballast', weight: '600 lb', arm: '40 in' }], envelope },
  ]) {
    const r = JSON.parse(await host.invoke('aviation.loading.weight-balance', JSON.stringify(args)));
    const d = diagram('aviation.loading.weight-balance', args, r);
    const ring = [...d.markup.matchAll(/<polygon class="dg-grid" points="([^"]+)"/g)][0][1]
      .split(' ')
      .map((p) => p.split(',').map(Number));
    assert.equal(ring.length, envelope.length, 'every envelope corner is drawn');
    const takeoff = circles(d.markup).find((c) => c.cls === 'dg-dot-now');
    // The drawing agrees with the core about whether the load is in limits.
    assert.equal(inside(ring, [takeoff.x, takeoff.y]), r.result.takeoff_status === 'inside', `${r.result.takeoff_status} point drawn on the wrong side`);
    // The verdict is in words too, not the position and color alone.
    assert.match(d.markup, new RegExp(`Takeoff .*\\(${r.result.takeoff_status}\\)`));
    assert.match(d.desc, new RegExp(r.result.takeoff_status));
    if (!r.result.landing_weight) continue;
    // Burning fuel moves the point along the drawn burn path to the landing dot.
    const landing = circles(d.markup).find((c) => c.cls === 'dg-dot');
    const path = lines(d.markup).find((l) => l.cls.includes('dg-dash'));
    assert.ok(Math.hypot(path.x1 - takeoff.x, path.y1 - takeoff.y) < 0.2, 'the burn path starts at the takeoff point');
    assert.ok(Math.hypot(path.x2 - landing.x, path.y2 - landing.y) < 0.2, 'and ends at the landing point');
    // Lighter after the burn, so the landing point is drawn higher up the weight axis.
    assert.ok(landing.y > takeoff.y, 'the landing point is lower in weight');
  }
});

test('the standard-atmosphere chart marks the point on the profile it draws', async () => {
  // add-aviation-suite 1.6 fixture.
  let last = -Infinity;
  for (const altitude of ['0 ft', '8000 ft', '18000 ft', '38000 ft']) {
    const args = { altitude };
    const r = JSON.parse(await host.invoke('aviation.atmosphere.isa', JSON.stringify(args)));
    const d = diagram('aviation.atmosphere.isa', args, r);
    const profile = [...d.markup.matchAll(/<path class="dg-muted" d="([^"]+)"/g)][0][1]
      .split(/[ML]/)
      .filter(Boolean)
      .map((p) => p.trim().split(' ').map(Number));
    const mark = circles(d.markup).find((c) => c.cls === 'dg-dot');
    // The marked temperature and altitude lie on the standard profile: in the
    // troposphere on the sloping leg, above it on the isothermal one.
    const [a, b] = mark.y < profile[1][1] ? [profile[1], profile[2]] : [profile[0], profile[1]];
    const at = (b[1] - a[1]) === 0 ? a[0] : a[0] + ((b[0] - a[0]) * (mark.y - a[1])) / (b[1] - a[1]);
    assert.ok(Math.abs(at - mark.x) < 1.5, `${altitude}: marked at x ${mark.x}, profile is at ${at}`);
    // Higher altitude, higher on the chart; the tropopause rule is drawn at the kink.
    assert.ok(mark.y <= last || last === -Infinity, `${altitude} drawn below a lower altitude`);
    last = mark.y;
    const kink = [...d.markup.matchAll(/<line class="dg-grid dg-dash"[^>]*y1="([-\d.]+)"/g)][0][1];
    assert.ok(Math.abs(Number(kink) - profile[1][1]) < 0.2, 'the tropopause rule sits at the kink');
    assert.match(d.desc, /Standard atmosphere/);
  }
});

test('the altimetry drawing puts true altitude below indicated in cold air', async () => {
  // add-aviation-suite 3.7 fixture.
  for (const args of [
    { indicated: '8000 ft', isa_deviation: '-20 degC' },
    { indicated: '8000 ft', isa_deviation: '25 degC' },
    { indicated: '2500 ft', isa_deviation: '-30 degC', station_elevation: '500 ft' },
  ]) {
    const r = JSON.parse(await host.invoke('aviation.altimetry.true-altitude', JSON.stringify(args)));
    const d = diagram('aviation.altimetry.true-altitude', args, r);
    const drawn = lines(d.markup).filter((l) => l.cls !== 'dg-casing');
    const indicated = drawn.find((l) => l.cls.includes('dg-dash'));
    const truth = drawn.find((l) => l.cls === 'dg-accent' && l.x1 === 150);
    const gap = drawn.find((l) => l.cls === 'dg-accent' && l.x1 === 170);
    const err = r.result.error.value;
    // Colder than standard, the aircraft is lower than the altimeter reads, so
    // the true line is drawn below the indicated one (y grows downward).
    assert.equal(truth.y1 > indicated.y1, err < 0, `${args.isa_deviation}: true line on the wrong side`);
    // The measured gap spans exactly the two, and both carry their values.
    assert.equal(gap.y1, indicated.y1);
    assert.equal(gap.y2, truth.y1);
    assert.ok(Math.abs(gap.y2 - gap.y1) > 20, 'the gap is legible, not a hairline');
    assert.ok(d.markup.includes(r.display.error), 'the gap is labeled with the error');
    assert.ok(d.markup.includes(r.display.true_altitude), 'the true altitude is labeled');
    // The window is a zoom, so the drawing says the axis is cut rather than
    // letting the reader scale the height off the ground line.
    assert.match(d.markup, /axis cut/);
    // The altimeter face reads the indicated altitude: its hundreds hand.
    const face = [...d.markup.matchAll(/<line class="dg-accent" x1="62(?:\.0)?" y1="120(?:\.0)?" x2="([-\d.]+)" y2="([-\d.]+)"/g)][0];
    const hundreds = (Math.atan2(Number(face[1]) - 62, 120 - Number(face[2])) * 180) / Math.PI;
    const want = ((Number.parseFloat(args.indicated) / 1000) % 1) * 360;
    assert.ok(Math.abs(((hundreds - want + 540) % 360) - 180) < 0.5, `hundreds hand at ${hundreds}, altitude wants ${want}`);
  }
});

test('the airspeed gauge marks each V-speed arc and names it in words', async () => {
  // add-aviation-suite 2.5 fixture, including the not-color-alone check.
  const marks = { vs0: '48 kt', vs1: '55 kt', vfe: '95 kt', vno: '130 kt', vne: '163 kt' };
  const args = { airspeed: '110 kt', pressure_altitude: '5000 ft', temperature: '5 degC', ...marks };
  const r = JSON.parse(await host.invoke('aviation.airspeed.cas-to-tas', JSON.stringify(args)));
  const d = diagram('aviation.airspeed.cas-to-tas', args, r);
  // The dial's own ticks give the scale: 0 at -135°, the top tick at +135°.
  const top = Math.max(...[...d.markup.matchAll(/<text class="dg-muted-text" text-anchor="middle"[^>]*>(\d+)</g)].map((m) => Number(m[1])));
  assert.ok(top >= 163, `the scale stops at ${top}, below VNE`);
  const speedAt = (x, y) => (((Math.atan2(x - 160, 108 - y) * 180) / Math.PI + 135) / 270) * top;
  const arcs = [...d.markup.matchAll(/<path class="(dg-arc-[a-z]+)" d="M([-\d.]+) ([-\d.]+)A[\d.]+ [\d.]+ 0 [01] 1 ([-\d.]+) ([-\d.]+)"/g)];
  const ends = Object.fromEntries(arcs.map((m) => [m[1], [speedAt(Number(m[2]), Number(m[3])), speedAt(Number(m[4]), Number(m[5]))]]));
  const kt = (s) => Number.parseFloat(marks[s]);
  // White is the flap range, green the normal range, yellow the caution range.
  for (const [cls, from, to] of [['dg-arc-white', 'vs0', 'vfe'], ['dg-arc-green', 'vs1', 'vno'], ['dg-arc-yellow', 'vno', 'vne']]) {
    assert.ok(ends[cls], `${cls} is not drawn`);
    assert.ok(Math.abs(ends[cls][0] - kt(from)) < 1, `${cls} starts at ${ends[cls][0]}, not ${from}`);
    assert.ok(Math.abs(ends[cls][1] - kt(to)) < 1, `${cls} ends at ${ends[cls][1]}, not ${to}`);
  }
  // The red line is at VNE, radial rather than an arc.
  const red = lines(d.markup).find((l) => l.cls === 'dg-arc-red');
  assert.ok(Math.abs(speedAt(red.x1, red.y1) - kt('vne')) < 1, `the red line is at ${speedAt(red.x1, red.y1)}`);
  // Not color alone: every arc says what it means.
  for (const word of ['Flaps', 'Normal', 'Caution', 'Never exceed']) assert.match(d.markup, new RegExp(`>${word}<`));
  // Without markings the gauge draws no arcs and claims no ranges.
  const plain = { airspeed: '110 kt', pressure_altitude: '5000 ft', temperature: '5 degC' };
  const bare = diagram('aviation.airspeed.cas-to-tas', plain, JSON.parse(await host.invoke('aviation.airspeed.cas-to-tas', JSON.stringify(plain))));
  assert.doesNotMatch(bare.markup, /dg-arc-/);
  assert.doesNotMatch(bare.markup, /Never exceed/);
});

test('the vector diagram chains the vectors head to tail to the sum', async () => {
  // add-navigation-and-geometry 3.5 fixture.
  for (const vectors of [
    [{ x: 3, y: 4 }, { x: -1, y: 2 }, { x: 2, y: -3 }],
    [{ x: -5, y: -2 }, { x: 1, y: 6 }],
    [{ x: 2, y: 1, z: 4 }, { x: -3, y: 2, z: -1 }],
  ]) {
    const args = { vectors };
    const r = JSON.parse(await host.invoke('navigation.vector.operations', JSON.stringify(args)));
    const d = diagram('navigation.vector.operations', args, r);
    const drawn = lines(d.markup).filter((l) => l.cls === 'dg-muted' || l.cls === 'dg-accent');
    const parts = drawn.filter((l) => l.cls === 'dg-muted');
    const sum = drawn.find((l) => l.cls === 'dg-accent');
    assert.equal(parts.length, vectors.length, 'one arrow per vector');
    // One scale for the whole drawing, taken from the first vector.
    const k = (parts[0].x2 - parts[0].x1) / vectors[0].x;
    for (const [i, v] of vectors.entries()) {
      assert.ok(Math.abs((parts[i].x2 - parts[i].x1) / k - v.x) < 0.02, `vector ${i} east component`);
      // North is up, so y runs the other way on screen.
      assert.ok(Math.abs((parts[i].y1 - parts[i].y2) / k - v.y) < 0.02, `vector ${i} north component`);
      if (i) assert.ok(Math.hypot(parts[i].x1 - parts[i - 1].x2, parts[i].y1 - parts[i - 1].y2) < 0.2, `vector ${i} does not start where ${i - 1} ended`);
    }
    // The resultant runs from the origin to the last head, at the direction
    // the core computed, clockwise from north.
    assert.ok(Math.hypot(sum.x1 - parts[0].x1, sum.y1 - parts[0].y1) < 0.2, 'the sum starts at the origin');
    assert.ok(Math.hypot(sum.x2 - parts.at(-1).x2, sum.y2 - parts.at(-1).y2) < 0.2, 'the sum ends at the last head');
    near(bearing(sum).deg, r.result.direction.value, 0.5, 'the sum direction');
    assert.ok(d.markup.includes(r.display.magnitude), 'the sum carries its magnitude');
    // A vertical component has no place on a plan view, so it is written out.
    if (vectors.some((v) => v.z)) assert.match(d.markup, new RegExp(`, ${vectors[0].z}\\)`));
  }
});

test('the ground profile flags the steep segments and states the exaggeration', async () => {
  // add-survey-suite 3.8 fixture.
  const args = {
    limit: 8,
    points: [
      { distance: '0 ft', elevation: '400 ft' },
      { distance: '100 ft', elevation: '406 ft' },
      { distance: '200 ft', elevation: '417 ft' },
      { distance: '300 ft', elevation: '414 ft' },
    ],
  };
  const r = JSON.parse(await host.invoke('survey.earthwork.profile-grades', JSON.stringify(args)));
  const d = diagram('survey.earthwork.profile-grades', args, r);
  const drawn = lines(d.markup).filter((l) => l.cls === 'dg-muted' || l.cls === 'dg-accent');
  assert.equal(drawn.length, r.result.segments.length, 'one line per segment');
  // The segment over the limit is the one drawn as such, and it says so in
  // words with its grade, not by color alone.
  for (const [i, seg] of r.result.segments.entries()) {
    assert.equal(drawn[i].cls === 'dg-accent', seg.over_limit === 'yes', `segment ${i}`);
    if (seg.over_limit === 'yes') assert.match(d.markup, new RegExp(`${seg.grade}% over limit`));
  }
  // Distance runs left to right and the segments join end to end.
  for (let i = 1; i < drawn.length; i += 1) {
    assert.ok(drawn[i].x1 > drawn[i - 1].x1, 'distance increases across the chart');
    assert.ok(Math.hypot(drawn[i].x1 - drawn[i - 1].x2, drawn[i].y1 - drawn[i - 1].y2) < 0.2, 'the profile is continuous');
  }
  // Rising ground climbs the chart, falling ground drops.
  for (const [i, seg] of r.result.segments.entries()) assert.equal(drawn[i].y2 < drawn[i].y1, seg.grade > 0, `segment ${i} direction`);
  // A profile is drawn with its heights stretched, so it says by how much.
  const stated = Number(/heights ×([\d.]+)/.exec(d.markup)[1]);
  const run = (drawn[0].x2 - drawn[0].x1) / 100;
  const rise = (drawn[0].y1 - drawn[0].y2) / 6;
  assert.ok(Math.abs(rise / run - stated) < 0.1, `states ×${stated}, draws ×${(rise / run).toFixed(1)}`);
});

test('the borrow pit draws the balance line through every crossing', async () => {
  // add-survey-suite 3.3 fixture.
  const args = {
    cell_size: '25 ft',
    existing: [{ elevations: '102.0, 101.5, 101.0' }, { elevations: '101.2, 100.6, 100.2' }, { elevations: '100.4, 99.8, 99.2' }],
    finished_grade: '100 ft',
  };
  const r = JSON.parse(await host.invoke('survey.earthwork.borrow-pit', JSON.stringify(args)));
  const d = diagram('survey.earthwork.borrow-pit', args, r);
  const marks = circles(d.markup).filter((c) => c.cls === 'dg-dot-now');
  assert.equal(marks.length, r.result.balance_points.length, 'every crossing is marked');
  const segs = lines(d.markup).filter((l) => l.cls.includes('dg-dash'));
  assert.equal(segs.length, marks.length - 1, 'the line joins the crossings');
  // The line is one chain: each segment starts where the last ended, and its
  // ends are marked crossings, not points of its own.
  for (const [i, seg] of segs.entries()) {
    if (i) assert.ok(Math.hypot(seg.x1 - segs[i - 1].x2, seg.y1 - segs[i - 1].y2) < 0.2, 'the balance line is continuous');
    for (const [x, y] of [[seg.x1, seg.y1], [seg.x2, seg.y2]]) {
      assert.ok(marks.some((m) => Math.hypot(m.x - x, m.y - y) < 0.2), 'a segment end is not a crossing');
    }
  }
  // The grid is drawn at the shape of the elevation table, and each node says
  // how deep the cut or fill is there.
  const grid = lines(d.markup).filter((l) => l.cls === 'dg-grid');
  assert.equal(grid.length, 3 + 3, 'a line per row and per column');
  for (const e of ['+2.0', '+1.5', '+1.0', '+0.4', '-0.2', '-0.8']) assert.ok(d.markup.includes(`>${e}<`), `node ${e} is not labeled`);
  // Where nothing crosses the grade there is no balance line, and the drawing
  // says so rather than leaving the reader to notice its absence.
  const allCut = { cell_size: '10 m', existing: [{ elevations: '5, 6' }, { elevations: '7, 8' }], finished_grade: '0 m' };
  const none = diagram('survey.earthwork.borrow-pit', allCut, JSON.parse(await host.invoke('survey.earthwork.borrow-pit', JSON.stringify(allCut))));
  assert.match(none.desc, /no balance line/);
  assert.equal(lines(none.markup).filter((l) => l.cls.includes('dg-dash')).length, 0);
});

test('the fly-by turn arc leaves the inbound leg and meets the outbound one', async () => {
  // add-navigation-and-geometry 2.8 turn-arc fixture.
  for (const args of [
    { inbound: '360 deg', outbound: '090 deg', speed: '120 kt', bank: '25 deg' },
    { inbound: '090 deg', outbound: '010 deg', speed: '200 kt', bank: '30 deg' },
    { inbound: '270 deg', outbound: '160 deg', speed: '150 kt', bank: '20 deg' },
  ]) {
    const r = JSON.parse(await host.invoke('navigation.route.fly-by', JSON.stringify(args)));
    const d = diagram('navigation.route.fly-by', args, r);
    const legs = lines(d.markup).filter((l) => l.cls === 'dg-muted');
    const [inbound, outbound] = legs;
    // The legs run along the courses, through the waypoint.
    near(bearing(inbound).deg, Number.parseFloat(args.inbound), 0.5, 'inbound leg');
    near(bearing(outbound).deg, Number.parseFloat(args.outbound), 0.5, 'outbound leg');
    const waypoint = circles(d.markup).find((c) => c.cls === 'dg-dot');
    assert.ok(Math.hypot(inbound.x2 - waypoint.x, inbound.y2 - waypoint.y) < 0.2, 'the inbound leg ends at the waypoint');
    assert.ok(Math.hypot(outbound.x1 - waypoint.x, outbound.y1 - waypoint.y) < 0.2, 'the outbound leg starts there');
    // The arc leaves the inbound leg and rejoins the outbound one, the same
    // distance either side of the waypoint, and sweeps the way the core says.
    const arc = /<path class="dg-accent" d="M([-\d.]+) ([-\d.]+) A([\d.]+) [\d.]+ 0 0 ([01]) ([-\d.]+) ([-\d.]+)"/.exec(d.markup);
    const [sx, sy, radius, sweep, ex, ey] = arc.slice(1).map(Number);
    assert.equal(sweep === 1, r.result.direction === 'right', `a ${r.result.direction} turn drawn the other way`);
    const lead = Math.hypot(sx - waypoint.x, sy - waypoint.y);
    assert.ok(Math.abs(Math.hypot(ex - waypoint.x, ey - waypoint.y) - lead) < 0.2, 'the turn is not symmetric about the waypoint');
    // Start and end sit on the legs, before and after the waypoint.
    near(bearing({ x1: sx, y1: sy, x2: waypoint.x, y2: waypoint.y }).deg, Number.parseFloat(args.inbound), 0.5, 'the turn starts on the inbound leg');
    near(bearing({ x1: waypoint.x, y1: waypoint.y, x2: ex, y2: ey }).deg, Number.parseFloat(args.outbound), 0.5, 'the turn ends on the outbound leg');
    // Tangent geometry: lead = radius × tan(half the turn), as the core has it.
    const half = Math.tan(((r.result.turn_angle.value ?? r.result.turn_angle) / 2) * (Math.PI / 180));
    assert.ok(Math.abs(lead / radius - half) < 0.02, `drawn lead/radius ${lead / radius}, tan(half turn) ${half}`);
    const coreRatio = r.result.lead_distance.value / r.result.radius.value;
    assert.ok(Math.abs(coreRatio - half) < 0.02, `the core's lead/radius is ${coreRatio}`);
  }
});
