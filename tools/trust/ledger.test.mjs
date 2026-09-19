// The freshness gates (trust/freshness): ledger completeness, superseded
// editions, overdue verification, model validity, and forward-only stamps.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { check as checkWith, readLedger } from './ledger.mjs';

const root = new URL('../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const today = new Date().toISOString().slice(0, 10);
const ledger = readLedger(root);
const check = (c, d) => checkWith(ledger, c, d);

test('every citation has a ledger row at its current edition, and nothing is overdue or expired', () => {
  const { errors, warnings } = check(catalog, today);
  if (warnings.length) console.warn(`Sources ledger warnings:\n  ${warnings.join('\n  ')}`);
  assert.deepEqual(errors, []);
});

test('the gates catch a new edition, an overdue check, and an expired model', () => {
  const asprs = ledger.find((r) => r.id === 'asprs-pas');
  const saved = { ...asprs };
  asprs.currentEdition = 'Edition 3';
  assert.ok(check(catalog, today).errors.some((e) => e.includes('current edition is Edition 3')));
  Object.assign(asprs, saved);
  const igrf = ledger.find((r) => r.id === 'igrf');
  assert.ok(check(catalog, '2030-01-15').errors.some((e) => e.startsWith('International Geomagnetic Reference Field: next edition was expected')));
  assert.ok(check(catalog, '2030-01-15').errors.some((e) => e.includes('WMM2025 expired')));
  assert.ok(check(catalog, '2029-06-01').warnings.some((w) => w.includes('WMM2025 is valid through 2029-12-31')));
  assert.equal(igrf.legalStatus, null);
});

test('a citation with no ledger row fails', () => {
  const fake = { tools: [{ id: 'x.y.z', references: [{ title: 'An Unknown Handbook', edition: '1st' }] }] };
  assert.ok(check(fake, today).errors[0].includes('no sources-ledger row'));
});

test('the spec-required sources are all in the ledger', () => {
  const ids = new Set(ledger.map((r) => r.id));
  for (const id of ['wmm', 'wmmhr', 'igrf', 'egm96', 'egm2008', 'geoid18', 'nsrs2022', 'nadcon5', 'spcs83', 'epsg', 'icao-7488', 'us76', 'icao-8168', 'aim', 'cfr-14-61', 'cfr-14-91',
    'cfr-14-107', 'cfr-14-89', 'part-108', 'eu-2019-947', 'eu-2019-945', 'asprs-pas', 'alta-nsps', 'usgs-lbs', 'h3', 's2', 'olc', 'a5', 'copernicus-dem', 'iers-bulletin-c']) {
    assert.ok(ids.has(id), id);
  }
  assert.equal(ledger.find((r) => r.id === 'part-108').legalStatus, 'proposed');
});

test('verification stamps only move forward against the base branch', () => {
  let base;
  try {
    base = JSON.parse(execFileSync('git', ['show', 'origin/main:data/sources-ledger.json'], { cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] })).sources;
  } catch {
    return; // no published ledger yet
  }
  for (const old of base) {
    const now = ledger.find((r) => r.id === old.id);
    if (now && old.lastVerified && (!now.lastVerified || now.lastVerified < old.lastVerified)) {
      assert.fail(`${old.id}: lastVerified moved back from ${old.lastVerified} to ${now.lastVerified}`);
    }
  }
});
