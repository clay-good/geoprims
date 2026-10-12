// data/regulations.json: every entry is complete, proposed rules stay labeled,
// and entries reviewed more than 12 months ago are listed for re-verification
// (a warning, per drone/operations-reference "Stale review date").
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const reg = JSON.parse(readFileSync(new URL('../../data/regulations.json', import.meta.url)));
const ledger = JSON.parse(readFileSync(new URL('../../data/sources-ledger.json', import.meta.url))).sources;
const STATUSES = new Set(['in-force', 'proposed', 'guidance']);
const DRONE_REFERENCE_IDS = [
  'faa-107-altitude',
  'faa-89-remote-id',
  'faa-108-nprm',
  'easa-open-height',
  'easa-open-mtom',
  'easa-open-a1',
  'easa-open-a2',
  'easa-open-a3',
  'easa-c1-energy',
  'easa-sts-classes',
];

const staleReviewEntries = (entries, now = Date.now()) => {
  const cutoff = now - 365 * 86_400_000;
  return entries.filter((entry) => Date.parse(entry.reviewed) < cutoff);
};

test('every regulation entry is complete and dated', () => {
  const ids = new Set();
  for (const e of reg.entries) {
    assert.ok(!ids.has(e.id), `duplicate id ${e.id}`);
    ids.add(e.id);
    for (const k of ['jurisdiction', 'citation', 'meaning', 'reviewed', 'status', 'url']) {
      assert.ok(e[k], `${e.id} needs ${k}`);
    }
    assert.ok(STATUSES.has(e.status), `${e.id}: unknown status ${e.status}`);
    assert.match(e.reviewed, /^\d{4}-\d{2}-\d{2}$/, `${e.id}: reviewed must be YYYY-MM-DD`);
    assert.ok(e.url.startsWith('https://'), `${e.id}: authority link must be https`);
  }
});

test('every regulation entry links to one sources-ledger row', () => {
  const rows = new Map(ledger.map((row) => [row.id, row]));
  assert.deepEqual(Object.keys(reg.sourceLinks).sort(), reg.entries.map((entry) => entry.id).sort());
  for (const entry of reg.entries) {
    const source = reg.sourceLinks[entry.id];
    const row = rows.get(source);
    assert.ok(row, `${entry.id}: unknown ledger source ${source}`);
    if (entry.status !== 'guidance') assert.equal(entry.status, row.legalStatus, `${entry.id}: status differs from ${source}`);
  }
});

test('Part 108 stays proposed until a final rule is published', () => {
  const e = reg.entries.find((x) => x.id === 'faa-108-nprm');
  assert.equal(e.status, 'proposed');
  assert.match(e.citation, /90 FR 38212/);
});

test('the initial drone regulation set covers FAA and every EASA class family', () => {
  const ids = new Set(reg.entries.map((entry) => entry.id));
  for (const id of DRONE_REFERENCE_IDS) assert.ok(ids.has(id), `missing required regulation entry ${id}`);
  assert.match(reg.entries.find((entry) => entry.id === 'easa-open-a1').meaning, /C0 and C1/);
  assert.match(reg.entries.find((entry) => entry.id === 'easa-open-a2').meaning, /C2/);
  assert.match(reg.entries.find((entry) => entry.id === 'easa-open-a3').meaning, /C2, C3, and C4/);
  assert.match(reg.entries.find((entry) => entry.id === 'easa-sts-classes').meaning, /C5.*C6/);
});

test('stale review dates are listed for re-verification', () => {
  const stale = staleReviewEntries(reg.entries).map((e) => `${e.id} (reviewed ${e.reviewed})`);
  if (stale.length) console.warn(`Re-verify these regulation entries (review older than 12 months):\n  ${stale.join('\n  ')}`);
  const fixture = staleReviewEntries([{ id: 'old-rule', reviewed: '2025-10-11' }], Date.parse('2026-10-12T00:00:00Z'));
  assert.deepEqual(fixture.map((entry) => entry.id), ['old-rule']);
});
