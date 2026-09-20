// The monthly correctness summary behind /quality (feedback/triage-and-
// corrections, "Feedback metrics without tracking"). It is built only from the
// curated known-issues file and the changelog, both of which are reviewed in
// pull requests. Nothing here comes from a visitor, and nothing is estimated:
// a figure the build cannot derive is reported as not yet measured.
import { issues } from './known-issues.mjs';
import { changelog } from './catalog.mjs';

const month = (date) => date.slice(0, 7);
const days = (from, to) => Math.round((Date.parse(to) - Date.parse(from)) / 86_400_000);

/** The median of a list of numbers, or null when the list is empty. */
export function median(values) {
  if (values.length === 0) return null;
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
}

/** Days from a confirmed defect being reported to the release that fixed it. */
export const timesToFix = () =>
  issues.filter((i) => i.status === 'fixed' && i.since && i.fixedOn).map((i) => days(i.since, i.fixedOn));

/** One row per month that saw a change, newest first. */
export function months() {
  const rows = new Map();
  const row = (m) => {
    if (!rows.has(m)) rows.set(m, { month: m, fixed: 0, resultChanges: 0, added: 0, changed: 0 });
    return rows.get(m);
  };
  for (const e of changelog.entries) {
    const r = row(month(e.date));
    if (e.kind === 'result-change') r.resultChanges += 1;
    else if (e.kind in r) r[e.kind] += 1;
  }
  for (const i of issues) {
    if (i.since) row(month(i.since)).reported = (row(month(i.since)).reported ?? 0) + 1;
  }
  return [...rows.values()].sort((a, b) => b.month.localeCompare(a.month));
}

/** The headline numbers, with nulls where the build cannot know yet. */
export function summary() {
  const fixed = issues.filter((i) => i.status === 'fixed');
  const open = issues.filter((i) => i.status !== 'fixed');
  const times = timesToFix();
  return {
    confirmedOpen: open.filter((i) => i.status === 'confirmed').length,
    investigating: open.filter((i) => i.status === 'investigating').length,
    fixed: fixed.length,
    medianDaysToFix: median(times),
    resultChanges: changelog.entries.filter((e) => e.kind === 'result-change').length,
    fixes: changelog.entries.filter((e) => e.kind === 'fixed').length,
    // Reports received is a count only the report Worker can give, and only
    // once reporting is switched on. Until then it is honestly absent.
    reportsReceived: null,
  };
}
