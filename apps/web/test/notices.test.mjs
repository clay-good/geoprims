// Notice stacking and priority (contracts/page-chrome). At most two notes show
// in full above the answer; the rest collapse into one line, so the answer card
// stays on the first screen of a phone.
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { EXPIRING_DAYS, RANKS, VISIBLE, noticesFor, split } from '../src/lib/notices.mjs';

const web = new URL('..', import.meta.url).pathname;
const dist = join(web, 'dist');
const TODAY = '2026-09-20';

const issue = { id: 'gh-12', summary: 'the gust column is read as knots', workaround: 'enter the gust yourself' };
const expiring = { id: 'wmm', name: 'World Magnetic Model', currentEdition: 'WMM2025', validTo: '2026-12-31' };
const expired = { id: 'egm', name: 'EGM96', currentEdition: '1996', validTo: '2020-01-01' };
const change = { date: '2026-08-15', kind: 'result-change', summary: 'the geoid height moved by 2 cm' };

test('the ranking is the contract\'s, highest priority first', () => {
  const notices = noticesFor({
    tool: { stability: 'experimental' },
    issue,
    changes: [change],
    sources: [expired, expiring],
    today: TODAY,
  });
  assert.deepEqual(notices.map((n) => n.kind), ['known-issue', 'expired-model', 'experimental', 'result-change', 'expiring-model']);
  for (let i = 1; i < notices.length; i++) assert.ok(RANKS[notices[i - 1].kind] <= RANKS[notices[i].kind]);
});

test('the many-notices scenario: two in full, the rest behind "N more notes"', () => {
  const notices = noticesFor({ tool: { stability: 'experimental' }, issue, changes: [change], sources: [expired], today: TODAY });
  assert.equal(notices.length, 4);
  const { shown, rest } = split(notices);
  assert.deepEqual(shown.map((n) => n.kind), ['known-issue', 'expired-model']);
  assert.equal(rest.length, 2, 'the page reads "2 more notes"');
  assert.equal(shown.length, VISIBLE);
});

test('a model is called out only once it is close to running out', () => {
  const soon = noticesFor({ tool: {}, issue: null, sources: [expiring], today: TODAY });
  assert.deepEqual(soon.map((n) => n.kind), ['expiring-model']);
  const farOff = noticesFor({ tool: {}, issue: null, sources: [{ ...expiring, validTo: '2029-12-31' }], today: TODAY });
  assert.deepEqual(farOff, [], `nothing is due within ${EXPIRING_DAYS} days`);
  const gone = noticesFor({ tool: {}, issue: null, sources: [expired], today: TODAY });
  assert.equal(gone[0].kind, 'expired-model');
  assert.match(gone[0].text, /ran out on 2020-01-01/);
});

test('a result change drops off the page after 90 days', () => {
  const fresh = noticesFor({ tool: {}, issue: null, changes: [change], today: TODAY });
  assert.deepEqual(fresh.map((n) => n.kind), ['result-change']);
  const old = noticesFor({ tool: {}, issue: null, changes: [{ ...change, date: '2026-01-01' }], today: TODAY });
  assert.deepEqual(old, []);
  const notAResultChange = noticesFor({ tool: {}, issue: null, changes: [{ ...change, kind: 'fixed' }], today: TODAY });
  assert.deepEqual(notAResultChange, []);
});

function htmlFiles(dir, out = []) {
  for (const f of readdirSync(dir)) {
    const p = join(dir, f);
    if (statSync(p).isDirectory()) htmlFiles(p, out);
    else if (f === 'index.html') out.push(p);
  }
  return out;
}

test('no built page shows more than two notices in full', () => {
  for (const file of htmlFiles(dist)) {
    const html = readFileSync(file, 'utf8');
    const full = [...html.matchAll(/<p class="card notice [a-z-]+" role="note">/g)].length;
    assert.ok(full <= VISIBLE, `${file.slice(dist.length)} shows ${full} notices in full`);
  }
});

test('an experimental tool says so above the answer, in plain words', () => {
  const html = readFileSync(join(dist, 'aviation/ifr/hold-entry/index.html'), 'utf8');
  const notice = /<p class="card notice experimental" role="note">([\s\S]*?)<\/p>/.exec(html);
  assert.ok(notice, 'the experimental notice is on the page');
  assert.match(notice[1], /Experimental: not yet fully verified\./);
  assert.ok(html.indexOf(notice[0]) < html.indexOf('<section class="card answer"'), 'notices come before the answer card');
});
