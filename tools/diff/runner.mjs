#!/usr/bin/env node
// The differential runner (platform/verification, "Cross-implementation
// differential testing"): for each tool family with an independent reference
// implementation installed, run seeded random and edge-biased cases through
// the tool (the same Wasm the site and MCP server ship) and through the
// reference, compare within the declared tolerance, and report the failures.
//
//   node tools/diff/runner.mjs [--cases 10000] [--family name] [--report dir]
//
// References are GeographicLib's command-line tools (GeodSolve, including its
// exact -E mode, RhumbSolve, GeoConvert, CartConvert, GeoidEval, IntersectTool). A family whose reference is missing is
// skipped and says so; the reference container (platform task 6.2) makes them
// all present in CI.
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { nodeHost } from '../../packages/runtime/src/node.mjs';

const root = new URL('../..', import.meta.url).pathname;
const GEOID_DIR = join(root, 'assets/data/egm96-15/2009-08-29');

/** mulberry32: a small seeded generator, so every run tests the same cases. */
export function rng(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const fx = (v) => v.toFixed(12); // GeographicLib reads an exponent's "e" as a hemisphere
/** A coordinate rounded to what the reference is sent, so both sides get the same number. */
const q = (v) => Number(fx(v));
const angDiff = (a, b) => Math.abs(((a - b + 540) % 360) - 180);
/** The direction a line s meters long has from nanometer-placed ends, in degrees. */
const azRounding = (s) => ((2e-9 / s) * 180) / Math.PI;
const uniformLat = (r) => (Math.asin(2 * r() - 1) * 180) / Math.PI;
const uniformLon = (r) => 360 * r() - 180;

/** A point anywhere, or 1 time in 5 at an edge: a pole, the equator, or the antimeridian. */
function point(r) {
  const k = r();
  if (k < 0.05) return [r() < 0.5 ? 90 : -90, uniformLon(r)];
  if (k < 0.1) return [0, uniformLon(r)];
  if (k < 0.15) return [uniformLat(r), r() < 0.5 ? 180 - 1e-9 * r() : -180 + 1e-9 * r()];
  if (k < 0.2) return [(r() < 0.5 ? 1 : -1) * (89.999 + 0.001 * r()), uniformLon(r)];
  return [uniformLat(r), uniformLon(r)];
}

/** A second point: anywhere, near-antipodal, close by, or the same point. */
function partner(r, p) {
  const k = r();
  const clamp = (lat) => Math.max(-90, Math.min(90, lat));
  if (k < 0.08) return [clamp(-p[0] + 0.01 * (r() - 0.5)), ((p[1] + 360) % 360) - 180 + 0.01 * (r() - 0.5)];
  if (k < 0.15) return [clamp(p[0] + 1e-6 * (r() - 0.5)), p[1] + 1e-6 * (r() - 0.5)];
  if (k < 0.17) return [...p];
  return point(r);
}

function reference(cmd, args, lines) {
  const out = spawnSync(cmd, args, { input: lines.join('\n') + '\n', encoding: 'utf8', maxBuffer: 1 << 28 });
  if (out.status !== 0) throw new Error(`${cmd} failed: ${out.stderr || out.stdout.split('\n').find((l) => /ERROR/.test(l))}`);
  return out.stdout.trim().split('\n').map((l) => l.trim().split(/\s+/).map(Number));
}

const has = (cmd) => {
  try {
    execFileSync(cmd, ['--version'], { stdio: 'ignore' });
    return true;
  } catch {
    return false;
  }
};

/**
 * The families. Each makes cases, runs its reference on them in one batch,
 * and compares one tool result with one reference row, returning the
 * failure's description or null.
 */
export const FAMILIES = [
  {
    name: 'geodesic-inverse',
    tool: 'navigation.geodesic.inverse',
    needs: 'GeodSolve',
    make(r) {
      const p = point(r).map(q);
      const t = partner(r, p).map(q);
      return { input: { lat1: p[0], lon1: p[1], lat2: t[0], lon2: t[1], options: { outputUnits: { distance: 'm' } } }, line: `${fx(p[0])} ${fx(p[1])} ${fx(t[0])} ${fx(t[1])}` };
    },
    run: (lines) => reference('GeodSolve', ['-i', '-p', '12'], lines),
    // |Δs| ≤ 1e-9 s + 15 nm and azimuths within 1e-9° (the spec's scenario),
    // plus what double-precision coordinates allow: they place a point to
    // about a nanometer, so a line s meters long has a direction only to about
    // 2 nm / s radians. An azimuth is undefined at a pole or between coincident points.
    compare(res, [az1, az2, s], c) {
      const d = res.distance.value;
      if (Math.abs(d - s) > 1e-9 * s + 15e-9) return `distance ${d} vs ${s}`;
      const defined = s > 1e-3 && Math.abs(c.input.lat1) < 90 && Math.abs(c.input.lat2) < 90;
      const tol = 1e-9 + azRounding(s);
      if (defined && angDiff(res.azimuth1.value, az1) > tol) return `azimuth1 ${res.azimuth1.value} vs ${az1}`;
      if (defined && angDiff(res.azimuth2.value, az2) > tol) return `azimuth2 ${res.azimuth2.value} vs ${az2}`;
      return null;
    },
  },
  {
    name: 'geodesic-direct',
    tool: 'navigation.geodesic.direct',
    needs: 'GeodSolve',
    make(r) {
      const p = point(r).map(q);
      const az = q(360 * r() - 180);
      const s = Number((r() < 0.1 ? 1e-3 * r() : 2e7 * r()).toFixed(9));
      return { input: { lat1: p[0], lon1: p[1], azimuth: `${az} deg`, distance: `${s} m` }, line: `${fx(p[0])} ${fx(p[1])} ${fx(az)} ${s.toFixed(9)}` };
    },
    run: (lines) => reference('GeodSolve', ['-p', '12'], lines),
    compare(res, [lat2, lon2]) {
      if (Math.abs(res.lat2.value - lat2) > 1e-9) return `lat2 ${res.lat2.value} vs ${lat2}`;
      if (Math.abs(lat2) < 90 - 1e-7 && angDiff(res.lon2.value, lon2) > 1e-9) return `lon2 ${res.lon2.value} vs ${lon2}`;
      return null;
    },
  },
  {
    name: 'rhumb-inverse',
    tool: 'navigation.rhumb.inverse',
    needs: 'RhumbSolve',
    make(r) {
      const p = point(r).map(q);
      const t = partner(r, p).map(q);
      return { input: { lat1: p[0], lon1: p[1], lat2: t[0], lon2: t[1], options: { outputUnits: { distance: 'm' } } }, line: `${fx(p[0])} ${fx(p[1])} ${fx(t[0])} ${fx(t[1])}` };
    },
    run: (lines) => reference('RhumbSolve', ['-i', '-p', '12'], lines),
    compare(res, [azi, s]) {
      const d = res.distance.value;
      if (Math.abs(d - s) > 1e-9 * s + 1e-6) return `distance ${d} vs ${s}`;
      // A rhumb's course comes from the difference of two nearly equal
      // meridional distances; both implementations lose a few nanometers of it
      // to cancellation, which on a centimeter line is ten times the plain rounding.
      if (s > 1e-3 && angDiff(res.course.value, azi) > 1e-8 + 10 * azRounding(s)) return `course ${res.course.value} vs ${azi}`;
      return null;
    },
  },
  {
    name: 'utm-forward',
    tool: 'geodesy.utm.forward',
    needs: 'GeoConvert',
    make(r) {
      // Edge-biased: 1 in 10 within a micrometer-scale step inside 80° S or 84° N.
      const lat = q(r() < 0.1 ? (r() < 0.5 ? -80 + 1e-6 * r() : 84 - 1e-6 * r()) : -80 + 164 * r());
      const lon = q(uniformLon(r));
      return { input: { lat, lon, options: { outputUnits: { easting: 'm', northing: 'm' } } }, line: `${fx(lat)} ${fx(lon)}` };
    },
    run: (lines) => reference('GeoConvert', ['-u', '-p', '9'], lines),
    // GeoConvert -u prints "zone+hemisphere easting northing" to the
    // nanometer; both evaluate the 6th-order Krüger series, good to a few
    // nanometers (Karney 2011), in different orders: 10 nm.
    compare(res, [, e, n]) {
      if (Math.abs(res.easting.value - e) > 1e-8) return `easting ${res.easting.value} vs ${e}`;
      if (Math.abs(res.northing.value - n) > 1e-8) return `northing ${res.northing.value} vs ${n}`;
      return null;
    },
  },
  {
    name: 'local-enu',
    tool: 'geodesy.frame.to-local',
    needs: 'CartConvert',
    make(r) {
      const o = point(r).map(q);
      const t = [Math.max(-90, Math.min(90, o[0] + 2 * (r() - 0.5))), o[1] + 2 * (r() - 0.5)].map(q);
      const [h0, h] = [5000 * r() - 100, 20000 * r() - 100].map((v) => Number(v.toFixed(9)));
      return {
        input: { lat0: o[0], lon0: o[1], h0: `${h0} m`, lat: t[0], lon: t[1], height: `${h} m`, options: { outputUnits: { east: 'm', north: 'm', up: 'm' } } },
        line: `${fx(t[0])} ${fx(t[1])} ${h.toFixed(9)}`,
        origin: [o[0], o[1], h0],
      };
    },
    batchKey: (c) => c.origin.join(','),
    run: (lines, c) => reference('CartConvert', ['-l', fx(c.origin[0]), fx(c.origin[1]), c.origin[2].toFixed(9), '-p', '9'], lines),
    compare(res, [e, n, u]) {
      for (const [k, want] of [['east', e], ['north', n], ['up', u]]) {
        if (Math.abs(res[k].value - want) > 1e-6) return `${k} ${res[k].value} vs ${want}`;
      }
      return null;
    },
  },
  {
    name: 'geoid-height',
    tool: 'geodesy.geoid.geoid-height',
    needs: 'GeoidEval',
    make(r) {
      const p = point(r).map(q);
      return { input: { lat: p[0], lon: p[1] }, line: `${fx(p[0])} ${fx(p[1])}` };
    },
    run: (lines) => reference('GeoidEval', ['-n', 'egm96-15', '-d', GEOID_DIR], lines),
    // GeoidEval prints to 0.1 mm.
    compare(res, [n]) {
      return Math.abs(res.geoid_height.value - n) > 6e-5 ? `geoid height ${res.geoid_height.value} vs ${n}` : null;
    },
  },
  {
    name: 'exact-inverse',
    tool: 'navigation.geodesic.inverse',
    needs: 'GeodSolve',
    // Strongly flattened ellipsoids take the exact method (0.02 < f ≤ 0.5); the
    // tool refuses nearly antipodal points there, so pairs stay within 150°.
    make(r) {
      const rf = [40, 10, 3, 2][Math.floor(4 * r())];
      let p;
      let t;
      do {
        p = [uniformLat(r), uniformLon(r)].map(q);
        t = [uniformLat(r), uniformLon(r)].map(q);
      } while (centralAngle(p, t) > 150);
      return {
        input: { lat1: p[0], lon1: p[1], lat2: t[0], lon2: t[1], a: '6378137 m', inverse_flattening: rf, options: { outputUnits: { distance: 'm' } } },
        line: `${fx(p[0])} ${fx(p[1])} ${fx(t[0])} ${fx(t[1])}`,
        rf,
      };
    },
    batchKey: (c) => String(c.rf),
    run: (lines, c) => reference('GeodSolve', ['-i', '-E', '-e', '6378137', `1/${c.rf}`, '-p', '12'], lines),
    compare(res, [az1, az2, s]) {
      if (Math.abs(res.distance.value - s) > 1e-9 * s + 15e-9) return `distance ${res.distance.value} vs ${s}`;
      const tol = 1e-9 + azRounding(s);
      if (angDiff(res.azimuth1.value, az1) > tol) return `azimuth1 ${res.azimuth1.value} vs ${az1}`;
      if (angDiff(res.azimuth2.value, az2) > tol) return `azimuth2 ${res.azimuth2.value} vs ${az2}`;
      return null;
    },
  },
  {
    name: 'mgrs-forward',
    tool: 'geodesy.grid-ref.mgrs-forward',
    needs: 'GeoConvert',
    // 1 m references (the default) and, 1 time in 4, millimeters; MGRS truncates.
    make(r) {
      const p = point(r).map(q);
      const mm = r() < 0.25;
      return { input: { lat: p[0], lon: p[1], ...(mm ? { precision: '0.001m' } : {}) }, line: `${fx(p[0])} ${fx(p[1])}`, mm };
    },
    batchKey: (c) => (c.mm ? 'mm' : 'm'),
    run: (lines, c) => referenceText('GeoConvert', ['-m', '-p', c.mm ? '3' : '0'], lines),
    compare(res, [want]) {
      return res.mgrs === want ? null : `mgrs ${res.mgrs} vs ${want}`;
    },
  },
  {
    name: 'segment-intersection',
    tool: 'navigation.geodesic.intersection',
    needs: 'IntersectTool',
    make(r) {
      const lat0 = 120 * r() - 60;
      const lon0 = uniformLon(r);
      const near = () => [q(Math.max(-85, Math.min(85, lat0 + 50 * (r() - 0.5)))), q(lon0 + 60 * (r() - 0.5))];
      const [a1, a2, b1, b2] = [near(), near(), near(), near()];
      return {
        input: { a_start_lat: a1[0], a_start_lon: a1[1], a_end_lat: a2[0], a_end_lon: a2[1], b_start_lat: b1[0], b_start_lon: b1[1], b_end_lat: b2[0], b_end_lon: b2[1] },
        line: [a1, a2, b1, b2].flat().map(fx).join(' '),
      };
    },
    run: (lines) => reference('IntersectTool', ['-i', '-p', '9'], lines),
    // IntersectTool -i prints x y c k: the distances along each segment,
    // coincidence, and 0 when the crossing is within both segments.
    compare(res, [x, y, c, k]) {
      if (c !== 0) return null;
      const tol = (v) => 1e-6 + 5e-12 * Math.abs(v);
      if (Math.abs(res.along_a.value - x) > tol(x)) return `along A ${res.along_a.value} vs ${x}`;
      if (Math.abs(res.along_b.value - y) > tol(y)) return `along B ${res.along_b.value} vs ${y}`;
      if ((res.within === 'yes') !== (k === 0)) return `within ${res.within} vs k = ${k}`;
      return null;
    },
  },
];

/** The central angle between two points on a sphere, degrees. */
function centralAngle(p, t) {
  const R = Math.PI / 180;
  const c = Math.sin(p[0] * R) * Math.sin(t[0] * R) + Math.cos(p[0] * R) * Math.cos(t[0] * R) * Math.cos((t[1] - p[1]) * R);
  return Math.acos(Math.max(-1, Math.min(1, c))) / R;
}

/** Like `reference`, for tools that print words (GeoConvert -m). */
function referenceText(cmd, args, lines) {
  const out = spawnSync(cmd, args, { input: lines.join('\n') + '\n', encoding: 'utf8', maxBuffer: 1 << 28 });
  if (out.status !== 0) throw new Error(`${cmd} failed: ${out.stderr || out.stdout}`);
  return out.stdout.trim().split('\n').map((l) => l.trim().split(/\s+/));
}

/**
 * Runs one family on `n` cases. `perturb`, when given, changes each tool
 * result before comparison (used to prove the comparison catches an error).
 */
export async function runFamily(host, fam, n, seed = 20260922, perturb = null) {
  const r = rng(seed);
  const cases = Array.from({ length: n }, () => fam.make(r));
  // Cases sharing a reference setup (like CartConvert's origin) go in one batch.
  const groups = new Map();
  for (const c of cases) {
    const k = fam.batchKey ? fam.batchKey(c) : '';
    if (!groups.has(k)) groups.set(k, []);
    groups.get(k).push(c);
  }
  const failures = [];
  let errors = 0;
  for (const group of groups.values()) {
    const rows = fam.run(group.map((c) => c.line), group[0]);
    for (let i = 0; i < group.length; i++) {
      const c = group[i];
      const env = JSON.parse(await host.invoke(fam.tool, JSON.stringify(c.input)));
      if (!env.ok) {
        errors += 1;
        failures.push({ input: c.input, reference: rows[i], problem: `${env.error.code}: ${env.error.message}` });
        continue;
      }
      const res = perturb ? perturb(env.result) : env.result;
      const why = fam.compare(res, rows[i], c);
      if (why) failures.push({ input: c.input, reference: rows[i], problem: why });
    }
  }
  return { family: fam.name, tool: fam.tool, cases: n, failures, errors };
}

async function main() {
  const args = process.argv.slice(2);
  const opt = (k, d) => (args.includes(k) ? args[args.indexOf(k) + 1] : d);
  const n = Number(opt('--cases', 10000));
  const only = opt('--family', null);
  const dir = opt('--report', join(root, 'dist/diff'));
  const host = nodeHost(join(root, 'dist/wasm'));
  mkdirSync(dir, { recursive: true });
  let failed = 0;
  const summary = [];
  for (const fam of FAMILIES.filter((f) => !only || f.name === only)) {
    if (!has(fam.needs)) {
      console.log(`${fam.name}: skipped, ${fam.needs} is not installed`);
      summary.push({ family: fam.name, skipped: `${fam.needs} not installed` });
      continue;
    }
    const t = Date.now();
    const out = await runFamily(host, fam, n);
    const secs = ((Date.now() - t) / 1000).toFixed(1);
    console.log(`${fam.name}: ${out.cases} cases, ${out.failures.length} failures (${secs} s)`);
    writeFileSync(join(dir, `${fam.name}.json`), JSON.stringify({ ...out, failures: out.failures.slice(0, 50) }, null, 2));
    summary.push({ family: fam.name, tool: fam.tool, cases: out.cases, failures: out.failures.length });
    failed += out.failures.length;
  }
  writeFileSync(join(dir, 'summary.json'), JSON.stringify(summary, null, 2));
  if (failed) {
    console.error(`${failed} differential failures; see ${dir}`);
    process.exit(1);
  }
}

if (import.meta.url === `file://${process.argv[1]}`) await main();
