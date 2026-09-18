// data/regulations.json: every entry is complete, proposed rules stay labeled,
// and entries reviewed more than 12 months ago are listed for re-verification
// (a warning, per drone/operations-reference "Stale review date").
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const reg = JSON.parse(readFileSync(new URL('../../data/regulations.json', import.meta.url)));
const STATUSES = new Set(['in-force', 'proposed', 'guidance']);

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

test('Part 108 stays proposed until a final rule is published', () => {
  const e = reg.entries.find((x) => x.id === 'faa-108-nprm');
  assert.equal(e.status, 'proposed');
  assert.match(e.citation, /90 FR 38212/);
});

test('stale review dates are listed for re-verification', () => {
  const cutoff = Date.now() - 365 * 86_400_000;
  const stale = reg.entries.filter((e) => Date.parse(e.reviewed) < cutoff).map((e) => `${e.id} (reviewed ${e.reviewed})`);
  if (stale.length) console.warn(`Re-verify these regulation entries (review older than 12 months):\n  ${stale.join('\n  ')}`);
  assert.ok(true);
});
