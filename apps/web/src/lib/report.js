// Problem-report payload (feedback/problem-reports "Report payload", contracts/
// report-api). Pure: the dialog, its tests, and the Worker's validator agree on
// this exact shape. Only the listed fields, never an identifier.
import LIMITS from '../../../../data/report-limits.json' with { type: 'json' };

export { LIMITS };
/** The toolId of a report about a page rather than a tool: the home page, a hub, a policy page. */
export const SITE_ID = 'site';

// Control characters and bidi overrides would be rejected by the Worker; replace them.
const RANGES = [
  [0x00, 0x1f],
  [0x7f, 0x9f],
  [0x202a, 0x202e],
  [0x2066, 0x2069],
];
const isForbidden = (c) => RANGES.some(([a, b]) => c >= a && c <= b);
const clean = (s) => [...String(s)].map((ch) => (isForbidden(ch.codePointAt(0)) ? ' ' : ch)).join('');
const cut = (s, n) => (s.length > n ? s.slice(0, n - 1) + '…' : s);
const row = (field, label, value, unit) => ({
  field: cut(clean(field), LIMITS.fieldChars),
  label: cut(clean(label), LIMITS.labelChars),
  value: cut(clean(typeof value === 'string' ? value : JSON.stringify(value)), LIMITS.valueChars),
  unit: cut(clean(unit ?? ''), LIMITS.unitChars),
});

export const viewportClass = (width) => (width < 600 ? 'phone' : width < 1024 ? 'tablet' : 'desktop');

/**
 * A report about a page that is not a tool (contracts/report-api "Page
 * reports"): the page's path, the site's core version and build, the kind,
 * and the note. No inputs, outputs, or warnings, because there are none.
 */
export function buildSitePayload({ site, note, kind, display, pagePath, token = '' }) {
  const trimmed = clean(note ?? '').trim();
  return {
    apiVersion: LIMITS.apiVersion,
    toolId: SITE_ID,
    toolVersion: site.coreVersion,
    coreVersion: site.coreVersion,
    buildHash: site.buildHash,
    assetVersions: {},
    kind,
    pagePath: cut(pagePath.split('#')[0], LIMITS.pagePathChars),
    inputs: [],
    outputs: [],
    warnings: [],
    display,
    note: trimmed ? cut(trimmed, LIMITS.noteChars) : null,
    token,
  };
}

/**
 * Builds the report. `tool` is the page's client manifest (inputs, outputs,
 * version, buildHash, coreVersion); `args` the current inputs; `result` the
 * current envelope. Inputs marked x-private are never included, and outputs
 * then read "(withheld)" because they may derive from them.
 */
export function buildPayload({ tool, args, result, includeInputs, note, kind, display, pagePath, token = '' }) {
  const props = tool.inputs.properties;
  const priv = Object.keys(props).filter((k) => props[k]['x-private']);
  const inputs = includeInputs
    ? Object.entries(args)
        .filter(([k]) => !priv.includes(k))
        .slice(0, LIMITS.inputRows)
        .map(([k, v]) => row(k, props[k]?.title ?? k, v, typeof v === 'number' ? props[k]?.['x-unit'] : ''))
    : [];
  const outputs =
    includeInputs && result?.ok
      ? Object.entries(result.result)
          .slice(0, LIMITS.outputRows)
          .map(([k, v]) => {
            const title = tool.outputs.properties[k]?.title ?? k;
            if (priv.length) return row(k, title, '(withheld)', '');
            return v && typeof v === 'object' && 'value' in v ? row(k, title, v.value, v.unit) : row(k, title, v, '');
          })
      : [];
  const warnings = result?.ok ? result.meta.warnings.map((w) => w.code).slice(0, LIMITS.warningCodes) : [];
  const trimmed = clean(note ?? '').trim();
  return {
    apiVersion: LIMITS.apiVersion,
    toolId: tool.id,
    toolVersion: tool.version,
    coreVersion: tool.coreVersion,
    buildHash: tool.buildHash,
    assetVersions: Object.fromEntries((result?.ok ? result.meta.assets : []).map((a) => [a.id, a.version])),
    kind,
    // The permalink fragment holds only known input keys; without inputs, no fragment.
    pagePath: cut(includeInputs ? pagePath : pagePath.split('#')[0], LIMITS.pagePathChars),
    inputs,
    outputs,
    warnings,
    display,
    note: trimmed ? cut(trimmed, LIMITS.noteChars) : null,
    token,
  };
}

/**
 * How long a bot-check token may be reused (contracts/report-api). Someone who
 * spends six minutes writing a note posts with a fresh one.
 */
export const TOKEN_MAX_AGE_MS = 240_000;

/** True when a token may be reused as it is, rather than asked for again. */
export const tokenIsFresh = (token, issuedAt, now) => Boolean(token) && now - issuedAt < TOKEN_MAX_AGE_MS;

/**
 * The state the dialog opens in. Offline when the browser is offline, paused
 * when reporting is switched off or the config could not be read (`config` is
 * null), and otherwise ready for the bot check. Nothing is ever queued: an
 * offline report is copied by hand or not sent at all.
 */
export function openState({ online, config }) {
  if (!online) return 'offline';
  return config?.enabled ? 'ready' : 'paused';
}

/** The state after a send. The Worker answers every accepted report with 202. */
export const sendState = (status) => (status === 202 ? 'sent' : 'failed');

/** Plain text for "Copy report" (offline or paused): the payload without the token. */
export function reportText(payload) {
  const { token: _token, ...rest } = payload;
  return `geoprims problem report\n${JSON.stringify(rest, null, 2)}\n\nSend it from the page's "Report a problem" button when you are back online.`;
}
