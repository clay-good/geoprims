// Report validation, canonical form, and keyed hashes (feedback/problem-reports
// "Endpoint validation" and "Abuse controls without tracking", contracts/
// report-api). Pure functions: the Worker and the tests share them.
import LIMITS from '../../data/report-limits.json' with { type: 'json' };

export { LIMITS };

/** The exact key set of a report body (contracts/report-api "Submit endpoint"). */
export const KEYS = [
  'apiVersion',
  'toolId',
  'toolVersion',
  'coreVersion',
  'buildHash',
  'assetVersions',
  'kind',
  'pagePath',
  'inputs',
  'outputs',
  'warnings',
  'display',
  'note',
  'token',
];

const ROW_KEYS = ['field', 'label', 'value', 'unit'];
const DISPLAY_KEYS = ['theme', 'unitProfile', 'viewportClass'];
// C0 and C1 controls (except none) and bidirectional overrides and isolates.
const FORBIDDEN = /[\u0000-\u001f\u007f-\u009f\u202a-\u202e\u2066-\u2069]/;
const URL_LIKE = /(https?:\/\/|www\.|[a-z0-9-]+\.(com|net|org|io|ru|cn|xyz|info|biz)\b)/i;

const isStr = (v, max) => typeof v === 'string' && v.length <= max && !FORBIDDEN.test(v);
const exactKeys = (o, keys) =>
  o !== null && typeof o === 'object' && !Array.isArray(o) && Object.keys(o).length === keys.length && keys.every((k) => Object.hasOwn(o, k));

function rowsOk(rows, max) {
  return (
    Array.isArray(rows) &&
    rows.length <= max &&
    rows.every(
      (r) =>
        exactKeys(r, ROW_KEYS) &&
        isStr(r.field, LIMITS.fieldChars) &&
        isStr(r.label, LIMITS.labelChars) &&
        isStr(r.value, LIMITS.valueChars) &&
        isStr(r.unit, LIMITS.unitChars),
    )
  );
}

/** The route of a tool id (contracts/routes-and-urls): /domain/group/op/. */
export const routeOf = (toolId) => `/${toolId.split('.').join('/')}/`;

/**
 * Validates a parsed body strictly. Returns the D1 row values on success or
 * null. `tools` is a Map of tool id → tool version from the bundled catalog.
 */
export function validate(body, tools) {
  if (!exactKeys(body, KEYS)) return null;
  if (body.apiVersion !== LIMITS.apiVersion) return null;
  if (!isStr(body.toolId, 80) || !tools.has(body.toolId)) return null;
  for (const k of ['toolVersion', 'coreVersion', 'buildHash']) if (!isStr(body[k], 64) || !body[k]) return null;
  if (!LIMITS.kinds.includes(body.kind)) return null;
  if (!isStr(body.pagePath, LIMITS.pagePathChars)) return null;
  const route = routeOf(body.toolId);
  const path = body.pagePath.split('#')[0];
  if (path !== route) return null;
  if (!rowsOk(body.inputs, LIMITS.inputRows) || !rowsOk(body.outputs, LIMITS.outputRows)) return null;
  if (!Array.isArray(body.warnings) || body.warnings.length > LIMITS.warningCodes || !body.warnings.every((w) => typeof w === 'string' && /^[A-Z][A-Z0-9_]{0,39}$/.test(w))) {
    return null;
  }
  if (!exactKeys(body.display, DISPLAY_KEYS) || !DISPLAY_KEYS.every((k) => isStr(body.display[k], 24))) return null;
  if (!['phone', 'tablet', 'desktop'].includes(body.display.viewportClass)) return null;
  if (body.note !== null && !isStr(body.note, LIMITS.noteChars)) return null;
  if (body.assetVersions === null || typeof body.assetVersions !== 'object' || Array.isArray(body.assetVersions)) return null;
  if (!Object.entries(body.assetVersions).every(([k, v]) => isStr(k, 40) && isStr(v, 60))) return null;
  if (typeof body.token !== 'string' || body.token.length > 4096) return null;
  const row = {
    tool_id: body.toolId,
    // The tool version is re-derived from the bundled catalog, not trusted.
    tool_version: tools.get(body.toolId),
    core_version: body.coreVersion,
    build_hash: body.buildHash,
    asset_versions_json: JSON.stringify(body.assetVersions),
    kind: body.kind,
    page_path: body.pagePath,
    note: body.note === null || body.note.trim() === '' ? null : body.note.trim(),
    note_has_url: body.note !== null && URL_LIKE.test(body.note) ? 1 : 0,
    inputs_json: JSON.stringify(body.inputs),
    outputs_json: JSON.stringify(body.outputs),
    warnings_json: JSON.stringify(body.warnings),
    display_json: JSON.stringify(body.display),
  };
  if (
    row.asset_versions_json.length > LIMITS.assetVersionsJsonChars ||
    row.inputs_json.length > LIMITS.inputsJsonChars ||
    row.outputs_json.length > LIMITS.outputsJsonChars ||
    row.warnings_json.length > LIMITS.warningsJsonChars ||
    row.display_json.length > LIMITS.displayJsonChars
  ) {
    return null;
  }
  return row;
}

const hex = (buf) => [...new Uint8Array(buf)].map((b) => b.toString(16).padStart(2, '0')).join('');
const enc = new TextEncoder();

/** Daily dedupe key: SHA-256 of the UTC date and the canonical report (no token). */
export async function dedupeKey(date, row) {
  const canonical = JSON.stringify([
    date,
    row.tool_id,
    row.tool_version,
    row.core_version,
    row.build_hash,
    row.kind,
    row.page_path,
    row.note,
    row.inputs_json,
    row.outputs_json,
    row.warnings_json,
  ]);
  return hex(await crypto.subtle.digest('SHA-256', enc.encode(canonical)));
}

/** Keyed daily reporter hash (design R2): HMAC-SHA256(secret, date + ":" + address). */
export async function reporterKey(secret, date, address) {
  const key = await crypto.subtle.importKey('raw', enc.encode(secret), { name: 'HMAC', hash: 'SHA-256' }, false, ['sign']);
  return hex(await crypto.subtle.sign('HMAC', key, enc.encode(`${date}:${address}`)));
}
