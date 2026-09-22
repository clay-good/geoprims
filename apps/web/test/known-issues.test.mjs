// feedback/triage-and-corrections "Banner on affected tool": a confirmed,
// unfixed known issue puts a banner on its tool's page, "Known issue:
// <summary>. Workaround: …", linking to its /known-issues entry; an issue
// under investigation or fixed does not, and the curated file is validated.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { openIssueIn, validate } from '../src/lib/known-issues.mjs';
import { noticesFor } from '../src/lib/notices.mjs';

const base = {
  tool: 'aviation.altimetry.cold-temperature',
  summary: 'the 4% rule is shown for heights above 5,000 ft',
  description: 'The rule is only printed for reference.',
  affected: '1.0.0',
  workaround: 'use the ICAO correction',
  since: '2026-09-22',
};
const list = validate([
  { ...base, id: 'ki-1', status: 'investigating' },
  { ...base, id: 'ki-2', status: 'confirmed' },
  { ...base, id: 'ki-3', status: 'fixed', fixedIn: '1.0.1', fixedOn: '2026-09-23' },
]);

test('the banner shows the confirmed issue, with its workaround and link', () => {
  const issue = openIssueIn(list, base.tool);
  assert.equal(issue.id, 'ki-2');
  const [banner] = noticesFor({ tool: { stability: 'stable' }, issue, today: '2026-09-22' });
  assert.equal(banner.kind, 'known-issue');
  assert.equal(banner.text, 'Known issue: the 4% rule is shown for heights above 5,000 ft. Workaround: use the ICAO correction');
  assert.equal(banner.href, '/known-issues/#ki-2');
});

test('investigating and fixed issues put no banner on the page', () => {
  assert.equal(openIssueIn(list.filter((i) => i.id !== 'ki-2'), base.tool), undefined);
  assert.equal(openIssueIn(list, 'aviation.altimetry.density-altitude'), undefined);
});

test('the curated file is refused when an entry is incomplete', () => {
  assert.throws(() => validate([{ ...base, id: 'x', status: 'confirmed', workaround: '' }]), /needs workaround/);
  assert.throws(() => validate([{ ...base, id: 'x', status: 'open' }]), /has status open/);
  assert.throws(() => validate([{ ...base, id: 'x', status: 'fixed' }]), /lacks fixedIn/);
});
