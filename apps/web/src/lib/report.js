// Problem-report payload (feedback/problem-reports "Report payload", contracts/
// report-api). Pure: the dialog, its tests, and the Worker's validator agree on
// this exact shape. Only the listed fields, never an identifier.
import LIMITS from '../../../../data/report-limits.json' with { type: 'json' };

export { LIMITS };
export const ISSUE_URL = 'https://github.com/clay-good/geoprims/issues/new?template=wrong-answer.yml';

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

/** Plain text for "Copy report" (offline or paused): the payload without the token. */
export function reportText(payload) {
  const { token: _token, ...rest } = payload;
  return `geoprims problem report\n${JSON.stringify(rest, null, 2)}\n\nSend it at ${ISSUE_URL}`;
}
