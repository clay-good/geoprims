// Practitioner review records (trust/correctness-program), parsed from
// docs/review-signoffs.md. Functions take the parsed record so the web build
// and the gates share one reader.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const VALID_DAYS = 365;

/** Rows of the markdown table under `## heading`, as objects keyed by its header. */
function table(md, heading) {
  const start = md.indexOf(`## ${heading}`);
  if (start < 0) return [];
  const rows = md.slice(start).split('\n').filter((l) => l.startsWith('|'));
  const cells = (l) => l.slice(1, -1).split('|').map((c) => c.trim());
  const [head, , ...body] = rows;
  if (!head) return [];
  const keys = cells(head).map((k) => k.toLowerCase());
  return body.map((l) => Object.fromEntries(cells(l).map((c, i) => [keys[i], c])));
}

export function parseSignoffs(md) {
  const required = Object.fromEntries(table(md, 'Reviewer needed per domain').map((r) => [r.domain, r.practitioner]));
  const records = table(md, 'Sign-offs').map((r) => ({ ...r, tools: r.tools.split(/,\s*/) }));
  return { required, records };
}

export function readSignoffs(root) {
  return parseSignoffs(readFileSync(join(root, 'docs/review-signoffs.md'), 'utf8'));
}

const ageDays = (date, today) => (Date.parse(today) - Date.parse(date)) / 86_400_000;

/** The current sign-off covering a tool id, or null. */
export function reviewFor(signoffs, id, today) {
  const domain = id.split('.')[0];
  return (
    signoffs.records.find(
      (r) => r.domain === domain && (r.tools.includes('all') || r.tools.includes(id)) && ageDays(r.date, today) <= VALID_DAYS,
    ) ?? null
  );
}

/** One plain sentence on a domain's (or a tool's) review status. */
export function reviewSentence(signoffs, domain, today, id = null) {
  const r = id
    ? reviewFor(signoffs, id, today)
    : signoffs.records.find((x) => x.domain === domain && x.tools.includes('all') && ageDays(x.date, today) <= VALID_DAYS);
  if (r) return `Reviewed by ${r.reviewer} (${r.qualification}) on ${r.date}: ${r.scope}.`;
  const who = signoffs.required[domain] ?? 'qualified practitioner';
  const article = /^[aeiou]/i.test(who) ? 'an' : 'a';
  return `Not yet independently reviewed by ${article} ${who}.`;
}

/** Problems with the records themselves: missing fields, unknown domains, expired rows. */
export function checkSignoffs(signoffs, catalog, today) {
  const problems = [];
  const ids = new Set(catalog.tools.map((t) => t.id));
  for (const d of new Set(catalog.tools.map((t) => t.domain))) {
    if (!signoffs.required[d]) problems.push(`domain ${d} has no required practitioner`);
  }
  for (const r of signoffs.records) {
    for (const k of ['domain', 'reviewer', 'qualification', 'date', 'scope']) {
      if (!r[k]) problems.push(`a ${r.domain ?? ''} sign-off is missing ${k}`);
    }
    if (!/^\d{4}-\d{2}-\d{2}$/.test(r.date ?? '')) problems.push(`sign-off by ${r.reviewer}: date must be YYYY-MM-DD`);
    else if (ageDays(r.date, today) > VALID_DAYS) problems.push(`sign-off by ${r.reviewer} for ${r.domain} expired (over 12 months)`);
    for (const t of r.tools) {
      if (t !== 'all' && !ids.has(t)) problems.push(`sign-off by ${r.reviewer} names unknown tool ${t}`);
    }
  }
  return problems;
}
