// Bounds fuzzing (trust/correctness-program layer D): every tool, through the
// real Wasm modules, with seeded mutations of its worked example. Each call must
// return a well-formed envelope: a result with no null numbers, or a
// structured error from the closed code list. Never a trap (INTERNAL), a hang,
// or a malformed response. Failures print the minimized input.
//
// FUZZ_CASES=n raises the per-tool case count (default 40); FUZZ_TIMEOUT_MS the
// whole run's budget (default 600,000; a 1,000-case run needs about an hour).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { workerHost } from '../../packages/runtime/src/worker-host.mjs';

const root = new URL('../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const CASES = Number(process.env.FUZZ_CASES ?? 40);
const CODES = new Set(['INVALID_INPUT', 'OUT_OF_DOMAIN', 'UNIT_MISMATCH', 'DID_NOT_CONVERGE', 'DEGENERATE_GEOMETRY', 'ASSET_UNAVAILABLE', 'ASSET_INTEGRITY', 'LIMIT_EXCEEDED', 'NO_SOLUTION', 'UNSUPPORTED']);
// The MCP server's default per-call timeout: anything slower is a hang to users.
const SLOW_MS = 9_000;

function rng(seed) {
  let s = seed >>> 0;
  return () => {
    s = (s + 0x6d2b79f5) >>> 0;
    let t = s;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const EXTREME_NUMBERS = [0, -0, -1, 1e-12, -1e-12, 1e9, -1e9, 1e300, -1e300, 5e-324, 90, -90, 180, 360, 361, 1000000];
const BAD_STRINGS = ['', ' ', 'abc', 'NaN', 'Infinity', '-Infinity', '1e999', '5 furlongs', '12,34,56', '½', '‮1', 'x'.repeat(300), '999999999999999999999 kt', '--5'];

function mutateValue(schema, current, r) {
  const pick = (a) => a[Math.floor(r() * a.length)];
  if (schema.enum) return r() < 0.8 ? pick(schema.enum) : 'bogus';
  if (schema.type === 'array') {
    const rows = Array.isArray(current) ? current : [];
    const k = r();
    if (k < 0.2) return [];
    if (k < 0.35) return 'not a list';
    if (k < 0.5) return Array.from({ length: Math.min(60, (schema.maxItems ?? 60) + 1) }, (_, i) => rows[i % Math.max(rows.length, 1)] ?? {});
    // Mutate one cell in one row.
    const out = structuredClone(rows);
    if (!out.length) return out;
    const row = out[Math.floor(r() * out.length)];
    const cols = Object.keys(schema.items?.properties ?? {});
    if (cols.length) {
      const c = pick(cols);
      row[c] = mutateValue(schema.items.properties[c], row[c], r);
    }
    return out;
  }
  const k = r();
  if (k < 0.45) {
    const n = pick(EXTREME_NUMBERS);
    const unit = schema['x-unit'] && r() < 0.5 ? ` ${schema['x-unit']}` : '';
    return unit ? `${n}${unit}` : n;
  }
  if (k < 0.7) return pick(BAD_STRINGS);
  if (k < 0.85) return (r() - 0.5) * 10 ** Math.floor(r() * 8);
  return null;
}

/** A null or non-finite number anywhere in a result is a bug. */
function badNumbers(v, path = 'result') {
  if (v === null) return [path];
  if (typeof v === 'number') return Number.isFinite(v) ? [] : [path];
  if (Array.isArray(v)) return v.flatMap((x, i) => badNumbers(x, `${path}.${i}`));
  if (typeof v === 'object') return Object.entries(v).flatMap(([k, x]) => badNumbers(x, `${path}.${k}`));
  return [];
}

// Calls run in a worker with a timeout, so a hang is caught and named, not waited out.
let host;

async function problem(t, input) {
  const started = performance.now();
  const out = await host.invoke(t.id, JSON.stringify(input));
  const ms = performance.now() - started;
  let r;
  try {
    r = JSON.parse(out);
  } catch {
    return `not JSON: ${out.slice(0, 200)}`;
  }
  if (ms > SLOW_MS) return `took ${Math.round(ms)} ms`;
  if (r.ok === true) {
    if (typeof r.summary !== 'string' || !r.meta || !r.result) return 'a success envelope is missing summary, meta, or result';
    const bad = badNumbers(r.result);
    if (bad.length) return `null or non-finite numbers at ${bad.join(', ')}`;
    return null;
  }
  if (r.ok === false) {
    if (r.error?.code === 'LIMIT_EXCEEDED' && r.error.message.includes('timeout')) return 'hung past the 10 s timeout';
    if (!r.error || !CODES.has(r.error.code)) return `error code ${r.error?.code}: ${r.error?.message}`;
    if (!r.error.message) return 'an error without a message';
    return null;
  }
  return 'an envelope without ok';
}

/** Shrinks a failing input: drop keys, then restore example values, while it still fails. */
async function minimize(t, input, example, what) {
  let cur = { ...input };
  for (const k of Object.keys(cur)) {
    const trial = { ...cur };
    delete trial[k];
    if ((await problem(t, trial)) === what) cur = trial;
  }
  for (const k of Object.keys(cur)) {
    if (!(k in example)) continue;
    const trial = { ...cur, [k]: example[k] };
    if ((await problem(t, trial)) === what) cur = trial;
  }
  return cur;
}

test('every tool survives bounds fuzzing', { timeout: Number(process.env.FUZZ_TIMEOUT_MS ?? 600_000) }, async () => {
  host = workerHost(join(root, 'dist/wasm'), { timeoutMs: 10_000 });
  const failures = [];
  try {
  for (const [ti, t] of catalog.tools.entries()) {
    const example = (t.examples.find((e) => e.id === t['x-primary-example']) ?? t.examples[0]).input;
    const fields = Object.entries(t.inputs.properties).filter(([k]) => k !== 'options');
    const r = rng(1000 + ti);
    for (let c = 0; c < CASES; c++) {
      const input = structuredClone(example);
      const n = 1 + Math.floor(r() * Math.min(3, fields.length));
      for (let m = 0; m < n; m++) {
        const [k, schema] = fields[Math.floor(r() * fields.length)];
        const v = mutateValue(schema, input[k], r);
        if (v === null) delete input[k];
        else input[k] = v;
      }
      const what = await problem(t, input);
      if (what) {
        const small = await minimize(t, input, example, what);
        failures.push(`${t.id}: ${what}\n    input: ${JSON.stringify(small)}`);
        break; // one per tool is enough to act on
      }
    }
  }
  } finally {
    host.close();
  }
  assert.deepEqual(failures, [], `\n${failures.join('\n')}`);
});

// Defects the fuzzer found, pinned as regressions: each must return this
// structured error (it used to trap, return a non-finite number, or hang).
const REGRESSIONS = [
  ['aviation.altimetry.density-altitude', { altimeter: '29.80 inHg', elevation: '1000000 ft', temperature: '30 degC' }, 'OUT_OF_DOMAIN'],
  ['aviation.altimetry.density-altitude', { altimeter: 0.28822659235447645, elevation: '5000 ft', temperature: '30 degC', dew_point: 27.62 }, 'OUT_OF_DOMAIN'],
  ['aviation.altimetry.density-altitude', { altimeter: '29.80 inHg', elevation: '5000 ft', temperature: 2017.2, dew_point: '361 degC' }, 'OUT_OF_DOMAIN'],
  ['aviation.altimetry.pressure-altitude', { altimeter: '29.80 inHg', elevation: 1e300 }, 'OUT_OF_DOMAIN'],
  ['aviation.atmosphere.isa', { altitude: '10000 ft', temperature_deviation: 1e300 }, 'OUT_OF_DOMAIN'],
  ['aviation.performance.pivotal-altitude', { groundspeed: '1e+300 kt' }, 'OUT_OF_DOMAIN'],
  ['aviation.performance.turn', { tas: 1e300 }, 'OUT_OF_DOMAIN'],
  ['aviation.weather.fb-winds-decode', { report: '731960', levels: '1e999' }, 'INVALID_INPUT'],
  ['drone.power.max-payload', { mass: 5e-324, rotor_diameter: '9.4 in', rotors: 4, target_time: '20 min', usable_energy: '72 Wh' }, 'OUT_OF_DOMAIN'],
  ['drone.mission.corridor', { centerline: [{ lat: 40, lon: -105 }, { lat: 40.02, lon: -104.98 }], line_spacing: '52.5 m', width: 1e9 }, 'LIMIT_EXCEEDED'],
  ['geodesy.utm.inverse', { easting: '586309.953 m', hemisphere: 'N', northing: '4477770.428 m', zone: 17, inverse_flattening: 180, a: '180 m' }, 'OUT_OF_DOMAIN'],
  ['time.sun.position', { lat: 39.7392, lon: -104.9903, time: '2026-06-21T12:00-06:00', delta_t: '-1e+300 s' }, 'OUT_OF_DOMAIN'],
  ['survey.reduction.combined-factor', { ellipsoid_height: '90 ft', grid_scale: 0.99991, radius: -90 }, 'OUT_OF_DOMAIN'],
];

test('fuzzer regressions return structured errors', async () => {
  const quick = workerHost(join(root, 'dist/wasm'), { timeoutMs: 10_000 });
  const count = catalog.tools.find((t) => t.id === 'drone.photogrammetry.image-count').examples[0].input;
  const cases = [
    ...REGRESSIONS,
    ['drone.mission.survey-grid', { ...count, area: count.area.map((p, i) => (i === 1 ? { ...p, ring: 1e300 } : p)) }, 'INVALID_INPUT'],
    ['drone.photogrammetry.image-count', { ...count, overshoot: '1000000000 m' }, 'OUT_OF_DOMAIN'],
  ];
  try {
    for (const [id, input, code] of cases) {
      const r = JSON.parse(await quick.invoke(id, JSON.stringify(input)));
      assert.equal(r.error?.code, code, `${id} ${JSON.stringify(input)} → ${JSON.stringify(r.error ?? r.summary)}`);
    }
  } finally {
    quick.close();
  }
});
