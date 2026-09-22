// The 2D projections (web/map-canvas, "Canvas modes" and "Polar cap"):
// equirectangular and polar azimuthal equidistant beside Web Mercator, each
// inverting what it projects, and a 500 km cap around the North Pole drawn as
// a circle in the polar view and a closed cap on the globe.
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { forward, frame, inverse, POLAR_REACH, PROJECTION_NAMES } from '../src/lib/map/projection.js';
import { nodeHost } from '../../../packages/runtime/src/node.mjs';

const web = new URL('..', import.meta.url).pathname;
const host = nodeHost(join(web, '../../dist/wasm'));
const view = (mode, lon, lat, scale = 300) => ({ mode, lon, lat, scale, width: 800, height: 600 });

/** A 500 km geodesic circle about the North Pole, from the core's direct problem. */
async function polarCap() {
  const ring = [];
  for (let az = 0; az < 360; az += 10) {
    const r = JSON.parse(await host.invoke('navigation.geodesic.direct', JSON.stringify({ lat1: 90, lon1: 0, azimuth: `${az} deg`, distance: '500 km' })));
    assert.ok(r.ok, r.error?.message);
    ring.push([r.result.lon2.value, r.result.lat2.value]);
  }
  return ring;
}

test('polar cap: a 500 km circle on the North Pole is a circle in the polar view and a closed cap on the globe', async () => {
  const ring = await polarCap();
  const polar = frame('polar', ring, 800, 600);
  assert.equal(polar.lat, 90, 'centered on the North Pole');
  const radii = ring.map(([lon, lat]) => {
    const [x, y] = forward(polar, lon, lat);
    return Math.hypot(x - 400, y - 300);
  });
  const spread = Math.max(...radii) - Math.min(...radii);
  assert.ok(spread < 0.01, `a circle, not ${spread.toFixed(4)} px out of round`);
  assert.ok(Math.min(...radii) > 100, 'framed to fill the view');
  // On the globe framed on the cap, every point is on the near side: the ring closes.
  const globe = frame('globe', ring, 800, 600);
  assert.ok(ring.every(([lon, lat]) => forward(globe, lon, lat)), 'no point of the cap is hidden');
});

test('each 2D projection inverts what it projects', () => {
  const samples = [[-105, 40], [0, 0], [170, -35], [-179.5, 65], [30, 88]];
  for (const mode of ['map', 'equirect', 'polar', 'globe']) {
    const v = mode === 'polar' ? view(mode, -100, 90, 150) : view(mode, -40, 20, 120);
    for (const [lon, lat] of samples) {
      if (mode === 'map' && Math.abs(lat) > 85) continue;
      const p = forward(v, lon, lat);
      if (!p) continue;
      const [l2, p2] = inverse(v, p[0], p[1]);
      assert.ok(Math.abs(p2 - lat) < 1e-9 && Math.abs(((l2 - lon + 540) % 360) - 180) < 1e-9, `${mode} ${lon},${lat} -> ${l2},${p2}`);
    }
    assert.ok(PROJECTION_NAMES[mode], `${mode} has a readout name`);
  }
});

test('the polar view puts its central meridian toward the viewer and stops at its reach', () => {
  const north = view('polar', -100, 90);
  const [x, y] = forward(north, -100, 60);
  assert.ok(Math.abs(x - 400) < 1e-9 && y > 300, 'the central meridian points down from the North Pole');
  const south = view('polar', 20, -90);
  const [sx, sy] = forward(south, 20, -60);
  assert.ok(Math.abs(sx - 400) < 1e-9 && sy < 300, 'and up from the South Pole');
  assert.equal(forward(north, 0, 90 - POLAR_REACH - 1), null, 'beyond its reach, nothing is drawn');
  assert.ok(forward(north, 0, 90 - POLAR_REACH + 1));
  // A set of southern points frames on the South Pole.
  assert.equal(frame('polar', [[0, -70], [90, -75]], 800, 600).lat, -90);
});

test('equirectangular spaces parallels evenly', () => {
  const v = view('equirect', 0, 0, 100);
  const y = (lat) => forward(v, 0, lat)[1];
  assert.ok(Math.abs((y(0) - y(10)) - (y(70) - y(80))) < 1e-9);
});

test('an area frames on itself; a lone point keeps its context', () => {
  const cell = [[-80, 40.4], [-79.9, 40.4], [-79.9, 40.45], [-80, 40.45]];
  const loose = frame('map', cell, 800, 600);
  const tight = frame('map', cell, 800, 600, { tight: true });
  assert.ok(tight.scale > loose.scale * 20, 'a small area fills the view when tight');
  const w = (v) => forward(v, -79.9, 40.4)[0] - forward(v, -80, 40.4)[0];
  assert.ok(w(tight) > 300 && w(tight) < 800, `the area spans most of the view: ${w(tight)} px`);
});
