// The report Worker end to end on Node's built-in SQLite behind a D1-shaped
// shim, so the real migration's CHECK constraints and batches run. Covers the
// problem-reports and report-api scenarios that do not need Cloudflare itself.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { join } from 'node:path';
import { DatabaseSync } from 'node:sqlite';

const root = new URL('../..', import.meta.url).pathname;
if (!existsSync(join(root, 'worker/src/catalog-tools.json'))) execFileSync('node', [join(root, 'worker/scripts/prepare.mjs')]);
const { default: worker, cleanup, CEILINGS } = await import('../src/index.mjs');
const { LIMITS, validate } = await import('../src/report.mjs');
const MIGRATION = readFileSync(join(root, 'worker/migrations/0001_problem_reports.sql'), 'utf8');

/** A minimal D1: prepare/bind/first/all/run and an atomic batch. */
function d1() {
  const db = new DatabaseSync(':memory:');
  db.exec(MIGRATION);
  const stmt = (sql, args = []) => ({
    sql,
    args,
    bind: (...a) => stmt(sql, a),
    all: async () => ({ results: db.prepare(sql).all(...args), success: true }),
    first: async () => db.prepare(sql).get(...args) ?? null,
    run: async () => ({ success: true, meta: db.prepare(sql).run(...args) }),
    exec: () => ({ results: db.prepare(sql).all(...args), success: true }),
  });
  return {
    raw: db,
    prepare: (sql) => stmt(sql),
    batch: async (stmts) => {
      db.exec('BEGIN');
      try {
        const out = stmts.map((s) => s.exec());
        db.exec('COMMIT');
        return out;
      } catch (e) {
        db.exec('ROLLBACK');
        throw e;
      }
    },
  };
}

const env = (over = {}) => ({
  REPORTS_ENABLED: 'true',
  TURNSTILE_SITEKEY: 'site',
  TURNSTILE_SECRET: 'secret',
  REPORTER_KEY_SECRET: 'k',
  ALLOWED_ORIGINS: 'https://geoprims.com',
  DB: d1(),
  ...over,
});

let turnstile = { success: true, action: 'problem-report', hostname: 'geoprims.com' };
globalThis.fetch = async () => new Response(JSON.stringify(turnstile));

const report = (over = {}) => ({
  apiVersion: 1,
  toolId: 'aviation.altimetry.density-altitude',
  toolVersion: '1.0.0',
  coreVersion: '0.1.0',
  buildHash: '0123456789abcdef',
  assetVersions: {},
  kind: 'wrong-result',
  pagePath: '/aviation/altimetry/density-altitude/#v1:abc',
  inputs: [{ field: 'elevation', label: 'Field elevation', value: '5000', unit: 'ft' }],
  outputs: [{ field: 'density_altitude', label: 'Density altitude', value: '7932', unit: 'ft' }],
  warnings: ['DRY_AIR_ASSUMED'],
  display: { theme: 'light', unitProfile: 'aviation', viewportClass: 'phone' },
  note: 'Expected 7,900 ft per the POH chart',
  token: 'tok',
  ...over,
});

const post = (e, body, { ip = '203.0.113.7', origin = 'https://geoprims.com', type = 'application/json', now } = {}) =>
  worker.fetch(
    new Request('https://geoprims.com/api/reports', {
      method: 'POST',
      headers: { 'Content-Type': type, Origin: origin, 'CF-Connecting-IP': ip },
      body: typeof body === 'string' ? body : JSON.stringify(body),
    }),
    e,
    {},
    now ?? new Date('2026-09-18T12:00:00Z'),
  );

const rows = (e) => e.DB.raw.prepare('SELECT * FROM problem_reports').all();

test('config: 503 when paused, limits when on', async () => {
  const off = await worker.fetch(new Request('https://geoprims.com/api/reports/config'), { REPORTS_ENABLED: 'true' });
  assert.equal(off.status, 503);
  assert.deepEqual(await off.json(), { enabled: false });
  const on = await worker.fetch(new Request('https://geoprims.com/api/reports/config'), env());
  assert.equal(on.status, 200);
  assert.equal(on.headers.get('Cache-Control'), 'max-age=300');
  const c = await on.json();
  assert.equal(c.enabled, true);
  assert.deepEqual(c.limits, LIMITS);
});

test('wrong method is 405', async () => {
  const r = await worker.fetch(new Request('https://geoprims.com/api/reports'), env());
  assert.equal(r.status, 405);
});

test('a valid report is stored, with the uniform 202', async () => {
  const e = env();
  const r = await post(e, report());
  assert.equal(r.status, 202);
  assert.deepEqual(await r.json(), { ok: true });
  const [row] = rows(e);
  assert.equal(row.tool_id, 'aviation.altimetry.density-altitude');
  assert.equal(row.status, 'open');
  assert.equal(row.note_has_url, 0);
});

test('unknown tool: not stored, identical response', async () => {
  const e = env();
  const good = await (await post(e, report())).text();
  const bad = await post(e, report({ toolId: 'nosuch.tool.id', pagePath: '/nosuch/tool/id/' }));
  assert.equal(bad.status, 202);
  assert.equal(await bad.text(), good);
  assert.equal(rows(e).length, 1);
});

test('oversized body: 400 and nothing stored', async () => {
  const e = env();
  const r = await post(e, JSON.stringify({ pad: 'x'.repeat(LIMITS.bodyBytes) }));
  assert.equal(r.status, 400);
  assert.equal(rows(e).length, 0);
});

test('not JSON, wrong type, or foreign origin: 400', async () => {
  const e = env();
  assert.equal((await post(e, '{nope')).status, 400);
  assert.equal((await post(e, report(), { type: 'text/plain' })).status, 400);
  assert.equal((await post(e, report(), { origin: 'https://evil.example' })).status, 400);
});

test('duplicates and the per-reporter cap are silent', async () => {
  const e = env();
  const first = await (await post(e, report())).text();
  assert.equal(await (await post(e, report())).text(), first);
  assert.equal(rows(e).length, 1, 'duplicate not stored');
  for (let i = 0; i < 6; i++) await post(e, report({ note: `note ${i}` }));
  assert.equal(rows(e).length, CEILINGS.reporterAccepted, 'five accepted per reporter per day');
  // Another reporter still gets through.
  await post(e, report({ note: 'someone else' }), { ip: '198.51.100.9' });
  assert.equal(rows(e).length, CEILINGS.reporterAccepted + 1);
});

test('no address or reporter key in the reports table', async () => {
  const e = env();
  await post(e, report());
  const cols = e.DB.raw.prepare('PRAGMA table_info(problem_reports)').all().map((c) => c.name);
  assert.ok(!cols.some((c) => /ip|addr|reporter/.test(c)), cols.join(','));
  assert.ok(!JSON.stringify(rows(e)).includes('203.0.113.7'));
  const limits = e.DB.raw.prepare('SELECT * FROM report_limits').all();
  assert.ok(!JSON.stringify(limits).includes('203.0.113.7'), 'counters hold only the keyed hash');
});

test('a failed bot check is not stored', async () => {
  const e = env();
  turnstile = { success: true, action: 'other', hostname: 'geoprims.com' };
  assert.equal((await post(e, report())).status, 202);
  turnstile = { success: true, action: 'problem-report', hostname: 'geoprims.com' };
  assert.equal(rows(e).length, 0);
});

test('control and bidi characters are rejected; URLs in notes are flagged', async () => {
  const tools = new Map([['aviation.altimetry.density-altitude', '1.0.0']]);
  assert.equal(validate(report({ note: `bad ${String.fromCharCode(0x202e)} text` }), tools), null);
  assert.equal(validate(report({ note: `bad ${String.fromCharCode(7)}` }), tools), null);
  assert.equal(validate(report({ extra: 1 }), tools), null, 'exact key set');
  assert.equal(validate(report({ pagePath: '/aviation/altimetry/isa-temperature/' }), tools), null, 'page must match the tool');
  assert.equal(validate(report({ note: 'see https://example.com' }), tools).note_has_url, 1);
});

test('the worst-case payload fits the body cap and every CHECK', async () => {
  const s = (n) => 'x'.repeat(n);
  const rowMax = { field: s(LIMITS.fieldChars), label: s(LIMITS.labelChars), value: s(LIMITS.valueChars), unit: s(LIMITS.unitChars) };
  const body = report({
    inputs: Array(LIMITS.inputRows).fill(rowMax),
    outputs: Array(LIMITS.outputRows).fill(rowMax),
    warnings: Array(LIMITS.warningCodes).fill('A'.repeat(20)),
    note: s(LIMITS.noteChars),
    pagePath: '/aviation/altimetry/density-altitude/#' + 'v'.repeat(400),
  });
  const bytes = new TextEncoder().encode(JSON.stringify(body)).length;
  assert.ok(bytes <= LIMITS.bodyBytes, `${bytes} bytes`);
  const e = env();
  assert.equal((await post(e, body)).status, 202);
  assert.equal(rows(e).length, 1, 'stored within the CHECK constraints');
});

test('D1 CHECKs agree with the limits', () => {
  const db = d1().raw;
  const ins = (note) =>
    db
      .prepare(
        `INSERT INTO problem_reports (id, created_at, tool_id, tool_version, core_version, build_hash, asset_versions_json, kind, page_path, note, inputs_json, outputs_json, warnings_json, display_json, dedupe_key)
         VALUES (?, 'now', 't', '1', '1', 'b', '{}', 'other', '/', ?, '[]', '[]', '[]', '{}', ?)`,
      )
      .run(String(Math.random()), note, String(Math.random()));
  ins('x'.repeat(LIMITS.noteChars));
  assert.throws(() => ins('x'.repeat(LIMITS.noteChars + 1)), /CHECK/);
});

test('scheduled cleanup: old resolved reports and old counters go', async () => {
  const e = env();
  await post(e, report(), { now: new Date('2026-01-01T00:00:00Z') });
  e.DB.raw.exec(`UPDATE problem_reports SET status = 'fixed', fixed_in_version = '0.1.1', resolved_at = '2026-01-02T00:00:00Z'`);
  await post(e, report({ note: 'still open' }), { now: new Date('2026-01-01T00:00:00Z') });
  e.DB.raw.exec(`UPDATE problem_reports SET status = 'open', resolved_at = NULL WHERE note = 'still open'`);
  await cleanup(e, new Date('2026-09-18T00:00:00Z'));
  const left = rows(e);
  assert.equal(left.length, 1);
  assert.equal(left[0].note, 'still open', 'open reports never expire');
  assert.equal(e.DB.raw.prepare('SELECT count(*) AS n FROM report_limits').get().n, 0);
});

test('security headers on every response, and no cookies', async () => {
  const e = env();
  for (const r of [await post(e, report()), await worker.fetch(new Request('https://geoprims.com/api/reports/config'), e), await worker.fetch(new Request('https://geoprims.com/api/other'), e)]) {
    assert.equal(r.headers.get('Content-Security-Policy'), "default-src 'none'; sandbox");
    assert.match(r.headers.get('Cache-Control'), /no-store|max-age=300/);
    assert.ok(r.headers.get('Strict-Transport-Security'));
    assert.equal(r.headers.get('Set-Cookie'), null);
  }
});

test('wrangler config keeps logging off and reporting paused by default', () => {
  const text = readFileSync(join(root, 'worker/wrangler.jsonc'), 'utf8').replace(/^\s*\/\/.*$/gm, '');
  const cfg = JSON.parse(text);
  assert.equal(cfg.observability.enabled, false);
  assert.equal(cfg.observability.logs.enabled, false);
  assert.equal(cfg.observability.logs.invocation_logs, false);
  assert.equal(cfg.logpush, false);
  assert.equal(cfg.vars.REPORTS_ENABLED, 'false');
  assert.deepEqual(cfg.routes.map((r) => r.pattern), ['geoprims.com/api/reports*']);
});

test('the migration is generated from the limits file', () => {
  execFileSync('node', [join(root, 'tools/codegen/d1-migration.mjs'), '--check']);
});
