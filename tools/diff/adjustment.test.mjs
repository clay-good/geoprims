// The three survey adjustments against an independent numpy implementation
// (platform/verification, cross-implementation differential testing). The
// textbook these tools were specified from is not open, so this is what stands
// in for it: the same models solved again in numpy's dense linear algebra, and
// for the partition, the cut piece measured by the shoelace formula rather
// than resolved. Skipped where no Python with numpy is found.
import { spawnSync } from 'node:child_process';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { nodeHost } from '../../packages/runtime/src/node.mjs';
import { rng } from './runner.mjs';

const root = new URL('../..', import.meta.url).pathname;
const host = nodeHost(join(root, 'dist/wasm'));
const python = ['python3', '/usr/bin/python3'].find(
  (p) => spawnSync(p, ['-c', 'import numpy'], { stdio: 'ignore' }).status === 0,
);
const reference = (cases) => {
  const run = spawnSync(python, ['-I', join(root, 'tools/diff/adjustment.py')], {
    input: JSON.stringify(cases), encoding: 'utf8', maxBuffer: 1 << 26,
  });
  assert.equal(run.status, 0, run.stderr);
  return JSON.parse(run.stdout);
};
const invoke = async (id, input) => {
  const r = JSON.parse(await host.invoke(id, JSON.stringify(input)));
  assert.equal(r.ok, true, `${id}: ${JSON.stringify(r.error)}`);
  return r.result;
};
const num = (v) => (typeof v === 'number' ? v : v?.value);
/** A quantity string the tool parses back to the same number. */
const m = (v) => `${v.toFixed(6)} m`;

/** `n` levelling networks: a chain, a loop over it, and random spurs. */
function networks(count) {
  const r = rng(0x1e5e1);
  const out = [];
  for (let k = 0; k < count; k++) {
    const marks = 3 + Math.floor(r() * 6);
    const truth = [0];
    for (let i = 1; i <= marks; i++) truth.push(truth[i - 1] + (r() * 20 - 10));
    const weighting = ['length', 'setups', 'sigma'][k % 3];
    const obs = [];
    const add = (a, b) => {
      // A small random misclosure, which is what the adjustment distributes.
      const rise = truth[b] - truth[a] + (r() - 0.5) * 0.02;
      const row = { from: `M${a}`, to: `M${b}`, rise: m(rise) };
      const length = 0.5 + r() * 3;
      const setups = 2 + Math.floor(r() * 8);
      const sigma = 0.002 + r() * 0.006;
      if (weighting === 'length') row.length = `${length.toFixed(4)} km`;
      if (weighting === 'setups') row.setups = setups;
      if (weighting === 'sigma') row.sd = `${(sigma * 1000).toFixed(3)} mm`;
      obs.push({ row, length, setups, sigma, a, b, rise });
    };
    for (let i = 0; i < marks; i++) add(i, i + 1);
    add(marks, 0);
    for (let s = 0; s < 1 + Math.floor(r() * 3); s++) {
      const a = Math.floor(r() * (marks + 1));
      let b = Math.floor(r() * (marks + 1));
      if (b === a) b = (a + 1) % (marks + 1);
      add(a, b);
    }
    out.push({ weighting, obs, base: truth[0] });
  }
  return out;
}

test('every levelling network matches an independent numpy least squares', { skip: !python && 'no Python with numpy is installed' }, async () => {
  const nets = networks(60);
  const cases = [];
  const results = [];
  for (const net of nets) {
    const fixed = m(100 + net.base);
    const input = {
      fixed: [{ name: 'M0', elevation: fixed }],
      observations: net.obs.map((o) => o.row),
    };
    results.push(await invoke('survey.reduction.level-network', input));
    cases.push({
      // Every number the reference gets is the one the tool parsed, rounded
      // the same way, so a difference is the solver and not the input.
      fixed: { M0: Number.parseFloat(fixed) },
      weighting: net.weighting,
      observations: net.obs.map((o) => ({
        from: `M${o.a}`, to: `M${o.b}`,
        // The reference is given the same rounded numbers the tool parsed.
        rise: Number.parseFloat(o.row.rise),
        // The rounded length the tool parsed, for the same reason.
        length: o.row.length ? Number.parseFloat(o.row.length) : o.length,
        setups: o.setups,
        // The same rounded standard deviation the tool parsed: the weight is
        // 1/sigma squared, so a rounding here is doubled in the weight.
        sigma: o.row.sd ? Number.parseFloat(o.row.sd) / 1000 : o.sigma,
      })),
    });
  }
  const ref = reference({ levelNetwork: cases }).levelNetwork;
  let worstH = 0;
  let worstV = 0;
  for (const [i, got] of results.entries()) {
    const want = ref[i];
    assert.equal(num(got.degrees_of_freedom), want.degreesOfFreedom, `net ${i}: degrees of freedom`);
    for (const row of got.adjusted) {
      const h = Number.parseFloat(String(row.elevation));
      const w = want.adjusted[row.name];
      assert.ok(w !== undefined, `net ${i}: ${row.name} is not an unknown in the reference`);
      worstH = Math.max(worstH, Math.abs(h - w));
    }
    // The variance scales with the weights, which differ by orders of
    // magnitude between the three weighting modes, so it is compared relatively.
    worstV = Math.max(worstV, Math.abs(num(got.reference_variance) / want.referenceVariance - 1));
  }
  // The tool prints elevations to a tenth of a millimetre, so that rounding is
  // the floor on the comparison; the variance is dimensionless here.
  console.log(`level network vs numpy: ${nets.length} networks, worst elevation ${(worstH * 1000).toFixed(4)} mm, worst reference variance ${worstV.toExponential(2)} relative`);
  // The tool prints elevations to a tenth of a millimetre, which is the floor
  // on the comparison.
  assert.ok(worstH < 1e-4, `worst elevation ${worstH} m`);
  // Measured 2.2e-16: with every input matched, the two solvers agree to
  // double precision, so anything looser would hide a real difference.
  assert.ok(worstV < 1e-12, `worst reference variance ${worstV} relative`);
});

test('every Helmert fit matches an independent numpy Gauss-Newton', { skip: !python && 'no Python with numpy is installed' }, async () => {
  const r = rng(0x48e1);
  const AS = (180 * 3600) / Math.PI;
  const cases = [];
  const results = [];
  for (let k = 0; k < 40; k++) {
    const convention = k % 2 ? 'coordinate-frame' : 'position-vector';
    const n = 4 + Math.floor(r() * 6);
    // Points around a station, the way a local fit is set up.
    const base = [4027000 + r() * 2000, 305000 + r() * 4000, 4918000 + r() * 3000];
    const src = [];
    for (let i = 0; i < n; i++) src.push(base.map((c) => c + (r() - 0.5) * 3000));
    // A small seven-parameter difference, plus noise so the fit is redundant.
    const p = [r() * 2 - 1, r() * 2 - 1, r() * 2 - 1, (r() - 0.5) / AS, (r() - 0.5) / AS, (r() - 0.5) / AS, (r() - 0.5) * 2e-6];
    const sign = convention === 'position-vector' ? 1 : -1;
    const dst = src.map(([x, y, z]) => {
      const [tx, ty, tz, rx, ry, rz, ds] = p;
      const rot = [
        x + sign * (ry * z - rz * y),
        y + sign * (rz * x - rx * z),
        z + sign * (rx * y - ry * x),
      ];
      return rot.map((c, j) => [tx, ty, tz][j] + (1 + ds) * c + (r() - 0.5) * 0.02);
    });
    const q = (v) => Number(v.toFixed(4));
    const S = src.map((v) => v.map(q));
    const D = dst.map((v) => v.map(q));
    results.push(await invoke('survey.cogo.helmert-3d-fit', {
      convention,
      points: S.map((v, i) => ({
        name: `P${i + 1}`,
        x: m(v[0]), y: m(v[1]), z: m(v[2]),
        x_target: m(D[i][0]), y_target: m(D[i][1]), z_target: m(D[i][2]),
      })),
    }));
    cases.push({ source: S, target: D, convention });
  }
  const ref = reference({ helmert: cases }).helmert;
  const worst = { t: 0, rot: 0, scale: 0 };
  for (const [i, got] of results.entries()) {
    const want = ref[i];
    assert.equal(num(got.degrees_of_freedom), want.degreesOfFreedom, `fit ${i}: degrees of freedom`);
    for (const k of ['tx', 'ty', 'tz']) worst.t = Math.max(worst.t, Math.abs(num(got[k]) - want[k]));
    for (const k of ['rx', 'ry', 'rz']) worst.rot = Math.max(worst.rot, Math.abs(num(got[k]) - want[k] * AS));
    worst.scale = Math.max(worst.scale, Math.abs(num(got.scale) - want.scale * 1e6));
  }
  console.log(`helmert fit vs numpy: ${cases.length} fits, worst translation ${worst.t.toExponential(2)} m, rotation ${worst.rot.toExponential(2)} arcsec, scale ${worst.scale.toExponential(2)} ppm`);
  assert.ok(worst.t < 1e-6, `worst translation ${worst.t} m`);
  assert.ok(worst.rot < 1e-8, `worst rotation ${worst.rot} arcsec`);
  assert.ok(worst.scale < 1e-8, `worst scale ${worst.scale} ppm`);
});

test('every partition cuts the area it was asked for, measured independently', { skip: !python && 'no Python with numpy is installed' }, async () => {
  const r = rng(0xa17);
  const cases = [];
  const asked = [];
  for (let k = 0; k < 45; k++) {
    // A convex ring, so every method has a solution: angles in order around a
    // centre, radii jittered.
    const n = 4 + Math.floor(r() * 4);
    const ring = [];
    for (let i = 0; i < n; i++) {
      const a = (2 * Math.PI * i) / n + (r() - 0.5) * 0.2;
      const rad = 60 + r() * 90;
      ring.push([Number((200 + rad * Math.cos(a)).toFixed(4)), Number((200 + rad * Math.sin(a)).toFixed(4))]);
    }
    let total = 0;
    for (let i = 0; i < n; i++) {
      const [x1, y1] = ring[i];
      const [x2, y2] = ring[(i + 1) % n];
      total += x1 * y2 - x2 * y1;
    }
    total = Math.abs(total) / 2;
    const want = total * (0.15 + r() * 0.5);
    const method = ['parallel', 'through-point', 'bearing'][k % 3];
    const side = 1 + Math.floor(r() * n);
    const input = {
      points: ring.map(([e, nn]) => ({ easting: e, northing: nn })),
      area: `${want.toFixed(4)} ft2`,
      method,
      side,
    };
    if (method === 'through-point') {
      const [[x1, y1], [x2, y2]] = [ring[side - 1], ring[side % n]];
      const f = 0.3 + r() * 0.4;
      input.distance = `${(Math.hypot(x2 - x1, y2 - y1) * f).toFixed(4)} ft`;
    }
    if (method === 'bearing') input.bearing = `${(r() * 180).toFixed(3)} deg`;
    const got = JSON.parse(await host.invoke('survey.cogo.area-partition', JSON.stringify(input)));
    if (!got.ok) continue; // a geometry this method cannot cut; the tool refuses it
    const res = got.result;
    const corners = [...String(res.cut_piece).matchAll(/\d+/g)].map((mm) => Number(mm[0]) - 1);
    if (!corners.length) continue;
    cases.push({
      ring,
      corners,
      start: [num(res.start_easting), num(res.start_northing)],
      end: [num(res.end_easting), num(res.end_northing)],
    });
    asked.push({ want, reported: num(res.cut_area), total, method });
  }
  assert.ok(cases.length >= 30, `only ${cases.length} partitions solved`);
  const ref = reference({ partition: cases }).partition;
  let worstAsked = 0;
  let worstReported = 0;
  for (const [i, want] of asked.entries()) {
    const area = ref[i].cutArea;
    worstAsked = Math.max(worstAsked, Math.abs(area - want.want) / want.total);
    worstReported = Math.max(worstReported, Math.abs(area - want.reported) / want.total);
  }
  console.log(`area partition vs shoelace: ${cases.length} cuts, worst ${(worstAsked * 1e6).toFixed(3)} ppm of the parcel off the area asked for, ${(worstReported * 1e6).toFixed(3)} ppm off the area reported`);
  // The solve declares 1e-7 of the unit; a part per million of the parcel is
  // far looser than that and still catches a piece built from wrong corners.
  assert.ok(worstAsked < 1e-6, `worst ${worstAsked} of the parcel`);
  assert.ok(worstReported < 1e-6, `worst ${worstReported} of the parcel`);
});
