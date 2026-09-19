// The problem-report Worker (add-problem-reporting design R1): the only server
// code in geoprims, bound to /api/reports*. It performs no calculation, keeps
// no request logs, stores no address, and answers every acceptable request
// with the same 202.
import TOOLS from './catalog-tools.json' with { type: 'json' };
import { LIMITS, dedupeKey, reporterKey, validate } from './report.mjs';

/** Hard ceilings configuration can lower but never raise (spec "Abuse controls"). */
export const CEILINGS = { reporterAttempts: 10, globalAttempts: 500, reporterAccepted: 5, globalAccepted: 250 };

const TOOL_VERSIONS = new Map(TOOLS.map((t) => [t.id, t.version]));

const HEADERS = {
  'Content-Security-Policy': "default-src 'none'; sandbox",
  'Cache-Control': 'no-store',
  'Strict-Transport-Security': 'max-age=63072000; includeSubDomains; preload',
  'X-Content-Type-Options': 'nosniff',
  'Referrer-Policy': 'no-referrer',
};

const json = (status, body, extra = {}) =>
  new Response(JSON.stringify(body), { status, headers: { ...HEADERS, 'Content-Type': 'application/json', ...extra } });
const ACCEPTED = () => json(202, { ok: true });

export function configured(env) {
  return env.REPORTS_ENABLED === 'true' && !!env.TURNSTILE_SITEKEY && !!env.TURNSTILE_SECRET && !!env.REPORTER_KEY_SECRET && !!env.DB;
}

const cap = (env, name, ceiling) => {
  const v = Number.parseInt(env[name] ?? '', 10);
  return Number.isFinite(v) && v >= 0 ? Math.min(v, ceiling) : ceiling;
};

/** Reads at most `max` bytes of the body; null when it is larger (reading stops). */
async function readCapped(request, max) {
  if (!request.body) return new Uint8Array();
  const reader = request.body.getReader();
  const parts = [];
  let size = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    size += value.byteLength;
    if (size > max) {
      await reader.cancel();
      return null;
    }
    parts.push(value);
  }
  const out = new Uint8Array(size);
  let o = 0;
  for (const p of parts) {
    out.set(p, o);
    o += p.byteLength;
  }
  return out;
}

async function turnstileOk(env, token, hostnames) {
  if (!token) return false;
  const form = new FormData();
  form.append('secret', env.TURNSTILE_SECRET);
  form.append('response', token);
  try {
    const r = await fetch('https://challenges.cloudflare.com/turnstile/v0/siteverify', {
      method: 'POST',
      body: form,
      signal: AbortSignal.timeout(5000),
    });
    const v = await r.json();
    return v.success === true && v.action === 'problem-report' && hostnames.includes(v.hostname);
  } catch {
    return false;
  }
}

const utcDate = (now) => now.toISOString().slice(0, 10);

async function submit(request, env, now) {
  if (request.method !== 'POST') return json(405, { ok: false }, { Allow: 'POST' });
  const type = (request.headers.get('Content-Type') ?? '').split(';')[0].trim().toLowerCase();
  const origins = (env.ALLOWED_ORIGINS ?? 'https://geoprims.com').split(',').map((s) => s.trim());
  if (type !== 'application/json' || request.headers.get('Content-Encoding') || !origins.includes(request.headers.get('Origin') ?? '')) {
    return json(400, { ok: false });
  }
  const bytes = await readCapped(request, LIMITS.bodyBytes);
  if (bytes === null) return json(400, { ok: false });
  let body;
  try {
    body = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
  } catch {
    return json(400, { ok: false });
  }
  // From here on every outcome is the same 202.
  if (!configured(env)) return ACCEPTED();
  const row = validate(body, TOOL_VERSIONS);
  if (!row) return ACCEPTED();
  const date = utcDate(now);
  const address = request.headers.get('CF-Connecting-IP') ?? '';
  const reporter = await reporterKey(env.REPORTER_KEY_SECRET, date, address);
  const bump = (scope, subject, kind) =>
    env.DB.prepare(
      `INSERT INTO report_limits (bucket, scope, subject, kind, count) VALUES (?1, ?2, ?3, ?4, 1)
       ON CONFLICT (bucket, scope, subject, kind) DO UPDATE SET count = count + 1 RETURNING count`,
    ).bind(date, scope, subject, kind);
  const [ra, ga] = await env.DB.batch([bump('reporter', reporter, 'attempt'), bump('global', '*', 'attempt')]);
  if (ra.results[0].count > cap(env, 'REPORTER_ATTEMPTS', CEILINGS.reporterAttempts)) return ACCEPTED();
  if (ga.results[0].count > cap(env, 'GLOBAL_ATTEMPTS', CEILINGS.globalAttempts)) return ACCEPTED();
  const hosts = origins.map((o) => new URL(o).hostname);
  if (!(await turnstileOk(env, body.token, hosts))) return ACCEPTED();
  const id = crypto.randomUUID();
  const key = await dedupeKey(date, row);
  const acceptedCount = (scope, subject) =>
    `COALESCE((SELECT count FROM report_limits WHERE bucket = ?1 AND scope = '${scope}' AND subject = ${subject} AND kind = 'accepted'), 0)`;
  // One atomic batch: the conditional insert, then the accepted counters only if it landed.
  await env.DB.batch([
    env.DB.prepare(
      `INSERT OR IGNORE INTO problem_reports (id, created_at, tool_id, tool_version, core_version, build_hash,
         asset_versions_json, kind, page_path, note, note_has_url, inputs_json, outputs_json, warnings_json,
         display_json, dedupe_key)
       SELECT ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18
       WHERE ${acceptedCount('reporter', '?2')} < ?19 AND ${acceptedCount('global', "'*'")} < ?20`,
    ).bind(
      date,
      reporter,
      id,
      now.toISOString(),
      row.tool_id,
      row.tool_version,
      row.core_version,
      row.build_hash,
      row.asset_versions_json,
      row.kind,
      row.page_path,
      row.note,
      row.note_has_url,
      row.inputs_json,
      row.outputs_json,
      row.warnings_json,
      row.display_json,
      key,
      cap(env, 'REPORTER_ACCEPTED', CEILINGS.reporterAccepted),
      cap(env, 'GLOBAL_ACCEPTED', CEILINGS.globalAccepted),
    ),
    ...[
      ['reporter', reporter],
      ['global', '*'],
    ].map(([scope, subject]) =>
      env.DB.prepare(
        `INSERT INTO report_limits (bucket, scope, subject, kind, count)
         SELECT ?1, ?2, ?3, 'accepted', 1 WHERE EXISTS (SELECT 1 FROM problem_reports WHERE id = ?4)
         ON CONFLICT (bucket, scope, subject, kind) DO UPDATE SET count = count + 1`,
      ).bind(date, scope, subject, id),
    ),
  ]);
  return ACCEPTED();
}

function config(env) {
  if (!configured(env)) return json(503, { enabled: false });
  return json(200, { enabled: true, sitekey: env.TURNSTILE_SITEKEY, limits: LIMITS, apiVersion: LIMITS.apiVersion }, { 'Cache-Control': 'max-age=300' });
}

/** Daily retention job (spec "Retention"): resolved reports after 180 days, counters after 14. */
export async function cleanup(env, now) {
  const days = (n) => new Date(now.getTime() - n * 86_400_000).toISOString();
  await env.DB.batch([
    env.DB.prepare(`DELETE FROM problem_reports WHERE status NOT IN ('open','triaged','confirmed') AND resolved_at IS NOT NULL AND resolved_at < ?1`).bind(days(180)),
    env.DB.prepare(`DELETE FROM report_limits WHERE bucket < ?1`).bind(days(14).slice(0, 10)),
  ]);
}

export default {
  async fetch(request, env, _ctx, now = new Date()) {
    const { pathname } = new URL(request.url);
    if (pathname === '/api/reports/config') {
      return request.method === 'GET' ? config(env) : json(405, { ok: false }, { Allow: 'GET' });
    }
    if (pathname === '/api/reports') return submit(request, env, now);
    return json(404, { ok: false });
  },
  async scheduled(_event, env, ctx) {
    if (env.DB) ctx.waitUntil(cleanup(env, new Date()));
  },
};
