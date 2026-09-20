// One way of writing a citation (trust/citations, "Citations travel with
// results"). The page, the printed sheet, and the copied answer all render a
// reference through here, so a reader who copies a result carries the same
// citation the page showed — and the same short form the core puts in
// `meta.references`.

/** Publisher, title, edition, then where in it to look. */
export const shortCitation = (r) =>
  [r.issuer, r.title, r.edition].filter(Boolean).join(', ') + (r.locator ? `. ${r.locator}` : '') + '.';

/** Hosts that sell the document rather than give it away. */
export const PAID_HOSTS = ['store.icao.int', 'www.iso.org', 'webstore.iec.ch', 'www.astm.org', 'www.sae.org', 'techstreet.com'];

export const isPaid = (url) => {
  try {
    return PAID_HOSTS.some((h) => new URL(url).host === h || new URL(url).host.endsWith(`.${h}`));
  } catch {
    return false;
  }
};

/**
 * Where a reader can get this source, when the ledger knows: a free copy
 * somewhere else, or the plain fact that the only copy is sold. Nothing when
 * the citation's own link is already it. A link into a standards store is
 * never called free.
 */
export function freeAccess(ref, row) {
  const url = row?.freeAccessUrl;
  if (!url || url === ref.url) return isPaid(ref.url) ? { url: null, label: 'Paid only' } : null;
  if (isPaid(url)) return { url, label: 'Paid only' };
  return { url, label: 'Read free' };
}

/**
 * The full "copy with reference" block: the answer with its units, the inputs
 * it came from, the method, every citation with its locator, the versions that
 * produced it, the day it was copied, and the notice that it is not a legal
 * determination.
 */
export function withReference({ answer, result, tool, args, href, today }) {
  const lines = [`${tool.title}: ${answer}`];
  if (result.summary) lines.push(result.summary);
  const inputs = Object.entries(args ?? {}).map(([k, v]) => `  ${tool.inputs.properties[k]?.title ?? k}: ${format(v)}`);
  if (inputs.length) lines.push('', 'Inputs:', ...inputs);
  const meta = result.meta ?? {};
  if (meta.model) lines.push('', `Method: ${meta.model}`);
  const references = meta.references ?? [];
  if (references.length) lines.push('', 'Sources:', ...references.map((r) => `  ${shortCitation(r)}`));
  lines.push(
    '',
    `geoprims ${tool.id} ${tool.version}, core ${meta.coreVersion ?? tool.coreVersion}${today ? `, copied ${today}` : ''}`,
  );
  if (href) lines.push(href);
  lines.push('Planning and education aid. Not a legal survey determination and not for primary navigation.');
  return lines.join('\n');
}

/** An input value as the reader typed it, or as a quantity reads. */
const format = (v) =>
  v !== null && typeof v === 'object' && 'value' in v ? `${v.value} ${v.unit ?? ''}`.trim() : String(v);
