#!/usr/bin/env node
// The free-access link probe (trust/freshness "Free-access link probe"): asks
// for every sources-ledger freeAccessUrl and every citation URL in the catalog,
// and writes the issue body that lists the broken and redirected ones, each
// with the citation, the tools that cite it, and the HTTP status. The monthly
// workflow (.github/workflows/link-probe.yml) opens or updates the issue; a
// probe never fails a build.
//
// Usage: node tools/trust/link-probe.mjs [--out probe.md]   (after build:catalog)
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { readLedger, sourceMap } from './ledger.mjs';

/** Every URL to probe, each with the citations that use it and the tools that cite them. */
export function collectLinks(ledger, catalog) {
  const links = new Map();
  const add = (url, citation, tools) => {
    if (!url) return;
    const l = links.get(url) ?? { url, citations: new Set(), tools: new Set() };
    l.citations.add(citation);
    for (const t of tools) l.tools.add(t);
    links.set(url, l);
  };
  for (const t of catalog.tools) {
    for (const r of t.references ?? []) add(r.url, [r.title, r.edition].filter(Boolean).join(', '), [t.id]);
  }
  for (const { row, tools } of sourceMap(ledger, catalog).values()) {
    add(row.freeAccessUrl, `${row.name} (sources ledger: ${row.id})`, tools);
  }
  return [...links.values()].map((l) => ({ ...l, citations: [...l.citations].sort(), tools: [...l.tools].sort() }));
}

/** A redirect that is how the link is meant to work: a DOI resolving, or an archive snapshot settling. */
const expectedRedirect = (from, to) =>
  /^https?:\/\/(dx\.)?doi\.org\//.test(from) ||
  (from.startsWith('https://web.archive.org/') && to.startsWith('https://web.archive.org/')) ||
  to === from.replace(/^http:/, 'https:') || to === `${from}/`;

/** A redirect to a bot wall or a sign-in page: the page itself went unchecked. */
const walled = (to) => /^https?:\/\/unblock\.|login|authorize|signin|sign-in|formsauthentication/i.test(to);

/**
 * One link's state: ok, redirected (with where to), refused (403 or 429, or a
 * redirect to a bot wall or sign-in page: the page is unchecked rather than
 * known to be gone), or broken (any other 4xx or 5xx, or no answer).
 * Redirects are not followed, so a moved document shows as moved.
 */
export async function probe(url, { timeoutMs = 20_000, fetchImpl = fetch } = {}) {
  const ask = (method) =>
    fetchImpl(url, {
      method,
      redirect: 'manual',
      signal: AbortSignal.timeout(timeoutMs),
      headers: { 'user-agent': 'geoprims-link-probe (+https://geoprims.com/sources/)' },
    });
  // Some servers mishandle HEAD, with an error status or by dropping the
  // connection, so a failed HEAD asks again for the page itself.
  const answer = async () => {
    try {
      const res = await ask('HEAD');
      if (res.status < 400) return res;
    } catch {
      // Fall through to GET.
    }
    return ask('GET');
  };
  try {
    const res = await answer();
    const status = res.status;
    if (status >= 300 && status < 400) {
      const to = res.headers.get('location');
      const target = to ? new URL(to, url).href : '';
      if (expectedRedirect(url, target)) return { state: 'ok', status };
      return walled(target) ? { state: 'refused', status, to: target } : { state: 'redirected', status, to: target };
    }
    if (status === 403 || status === 429) return { state: 'refused', status };
    if (status >= 400) return { state: 'broken', status };
    return { state: 'ok', status };
  } catch (e) {
    return { state: 'broken', status: e?.name === 'TimeoutError' ? 'timeout' : 'no answer' };
  }
}

/** Probes every link, a few at a time, in a stable order. */
export async function probeAll(links, { concurrency = 6, ...opts } = {}) {
  const out = new Array(links.length);
  let next = 0;
  await Promise.all(Array.from({ length: Math.min(concurrency, links.length) }, async () => {
    while (next < links.length) {
      const i = next++;
      out[i] = { ...links[i], ...(await probe(links[i].url, opts)) };
    }
  }));
  return out;
}

/** The issue body, or '' when every link answered. */
export function report(results, { date } = {}) {
  const row = (r) => `| ${r.url} | ${r.status}${r.to ? ` → ${r.to}` : ''} | ${r.citations.join('; ')} | ${r.tools.map((t) => `\`${t}\``).join(', ') || 'none yet'} |`;
  const section = (title, note, rows) =>
    rows.length ? [`## ${title} (${rows.length})`, '', note, '', '| Link | Status | Citation | Tools |', '|---|---|---|---|', ...rows.map(row), ''] : [];
  const by = (s) => results.filter((r) => r.state === s);
  const [broken, moved, refused] = [by('broken'), by('redirected'), by('refused')];
  if (!broken.length && !moved.length && !refused.length) return '';
  return [
    `The monthly free-access probe${date ? ` on ${date}` : ''} checked ${results.length} links (trust/freshness "Free-access link probe").`,
    '',
    ...section('Broken', 'These did not answer, or answered with an error. Find where the document went and repoint the citation and the ledger row.', broken),
    ...section('Redirected', 'These answer by sending readers somewhere else. Check the new address is the same document, then cite it directly.', moved),
    ...section('Not checked', 'These sites refused an automated request or asked it to sign in, so the probe cannot say whether the page is still there. Check them by hand.', refused),
  ].join('\n');
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const root = new URL('../..', import.meta.url).pathname;
  const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
  const links = collectLinks(readLedger(root), catalog);
  const results = await probeAll(links);
  const body = report(results, { date: new Date().toISOString().slice(0, 10) });
  const out = process.argv.includes('--out') ? process.argv[process.argv.indexOf('--out') + 1] : null;
  if (out) writeFileSync(out, body);
  const count = (s) => results.filter((r) => r.state === s).length;
  console.log(`${links.length} links: ${count('ok')} ok, ${count('broken')} broken, ${count('redirected')} redirected, ${count('refused')} not checked`);
  if (!out) console.log(body);
}
