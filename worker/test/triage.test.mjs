// Triage wrappers against the real migration on Node's SQLite: the workflow
// allows only its own moves, records what each move requires, and the
// triage list puts wrong results first with overdue flags.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { transition, TRIAGE_QUERY, triageCutoffs, REPORT_QUERY } from '../src/triage.mjs';
import { inline, literal, parse, plan } from '../scripts/triage.mjs';

const root = new URL('../..', import.meta.url).pathname;
const MIGRATION = readFileSync(join(root, 'worker/migrations/0001_problem_reports.sql'), 'utf8');
const NOW = '2026-09-22T12:00:00.000Z';

function seeded() {
  const db = new DatabaseSync(':memory:');
  db.exec(MIGRATION);
  const add = db.prepare(`INSERT INTO problem_reports (id, created_at, tool_id, tool_version, core_version, build_hash,
    asset_versions_json, kind, page_path, note, inputs_json, outputs_json, warnings_json, display_json, dedupe_key)
    VALUES (?, ?, ?, '1.0.0', '0.1.0', 'abc', '{}', ?, ?, NULL, '[]', '[]', '[]', '{}', ?)`);
  for (const [id, at, kind] of [
    ['r1', '2026-09-22T10:00:00.000Z', 'confusing'],
    ['r2', '2026-09-21T06:00:00.000Z', 'wrong-result'],
    ['r3', '2026-09-22T11:00:00.000Z', 'wrong-result'],
    ['r4', '2026-09-18T11:00:00.000Z', 'broken'],
  ]) {
    add.run(id, at, 'aviation.altimetry.density-altitude', kind, '/aviation/altimetry/density-altitude/', `k-${id}`);
  }
  return db;
}

const move = (db, id, from, to, fields) => {
  const t = transition({ id, from, to, fields, now: NOW });
  if (t instanceof Error) return t;
  return db.prepare(t.sql).run(...t.params).changes;
};
const status = (db, id) => db.prepare('SELECT status, primary_source, fixed_in_version, resolution_note, resolved_at FROM problem_reports WHERE id = ?').get(id);

test('a report walks open → triaged → confirmed → fixed, recording the source and version', () => {
  const db = seeded();
  assert.equal(move(db, 'r2', 'open', 'triaged'), 1);
  assert.equal(move(db, 'r2', 'triaged', 'confirmed', { primarySource: 'ICAO Doc 7488, Table 1' }), 1);
  assert.equal(move(db, 'r2', 'confirmed', 'fixed', { fixedInVersion: '1.1.0' }), 1);
  const s = status(db, 'r2');
  assert.deepEqual([s.status, s.primary_source, s.fixed_in_version, s.resolved_at], ['fixed', 'ICAO Doc 7488, Table 1', '1.1.0', NOW]);
});

test('moves the workflow does not allow are refused before any SQL', () => {
  for (const [from, to, fields, why] of [
    ['triaged', 'confirmed', {}, /primary source/],
    ['triaged', 'confirmed', { primarySource: '   ' }, /primary source/],
    ['confirmed', 'fixed', {}, /version/],
    ['confirmed', 'fixed', { fixedInVersion: 'soon' }, /version/],
    ['open', 'not_a_bug', {}, /resolution note/],
    ['open', 'duplicate', {}, /resolution note/],
    ['open', 'confirmed', { primarySource: 'x' }, /next status/],
    ['open', 'fixed', { fixedInVersion: '1.0.1' }, /next status/],
    ['fixed', 'open', {}, /final/],
    ['open', 'closed', {}, /not a status/],
  ]) {
    const t = transition({ id: 'r1', from, to, fields, now: NOW });
    assert.ok(t instanceof Error, `${from} → ${to} should be refused`);
    assert.match(t.message, why);
  }
});

test('a stale move changes nothing: the old status must still hold', () => {
  const db = seeded();
  assert.equal(move(db, 'r1', 'triaged', 'wont_fix', { resolutionNote: 'as designed' }), 0);
  assert.equal(status(db, 'r1').status, 'open');
  assert.equal(move(db, 'r1', 'open', 'not_a_bug', { resolutionNote: "The rule of thumb isn't the exact value." }), 1);
  assert.equal(status(db, 'r1').resolution_note, "The rule of thumb isn't the exact value.");
});

test('the triage list puts wrong results first, oldest first, with overdue flags', () => {
  const db = seeded();
  const rows = db.prepare(TRIAGE_QUERY).all(...triageCutoffs(NOW));
  assert.deepEqual(rows.map((r) => [r.id, r.overdue]), [
    ['r2', 1], // wrong result, 30 hours old
    ['r3', 0], // wrong result, 1 hour old
    ['r4', 1], // broken, 4 days old
    ['r1', 0],
  ]);
});

test('the CLI inlines values safely and refuses what the rules refuse', () => {
  assert.equal(literal("O'Brien"), "'O''Brien'");
  assert.throws(() => literal('a\nb'), /control characters/);
  assert.throws(() => inline('? ?', ['a']), /1 values for 2/);
  const sql = plan(parse(['set', 'r2', '--from', 'triaged', '--to', 'confirmed', '--source', "Bowditch's table"]), NOW);
  assert.equal(sql, "UPDATE problem_reports SET status = 'confirmed', primary_source = 'Bowditch''s table' WHERE id = 'r2' AND status = 'triaged'");
  // The printed statement runs as is.
  const db = seeded();
  move(db, 'r2', 'open', 'triaged');
  assert.equal(db.prepare(sql).run().changes, 1);
  assert.ok(plan(parse(['set', 'r2', '--from', 'triaged', '--to', 'confirmed']), NOW) instanceof Error);
  assert.match(plan(parse(['show', "x' OR '1'='1"]), NOW), /WHERE id = 'x'' OR ''1''=''1'$/);
  assert.equal(db.prepare(plan(parse(['show', 'r3']), NOW)).all().length, 1);
  assert.equal(db.prepare(REPORT_QUERY).all('r3')[0].kind, 'wrong-result');
});
