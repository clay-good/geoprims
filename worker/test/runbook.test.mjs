// docs/runbooks/problem-reports.md: every SQL query in the runbook runs
// against the real migration with seeded reports, so a schema change that
// breaks one fails here instead of during an incident.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { DatabaseSync } from 'node:sqlite';

const root = new URL('../..', import.meta.url).pathname;
const runbook = readFileSync(join(root, 'docs/runbooks/problem-reports.md'), 'utf8');
const queries = [...runbook.matchAll(/```sql\n([\s\S]*?)```/g)].map((m) => m[1].trim());

test('every runbook query runs on a seeded database', () => {
  const db = new DatabaseSync(':memory:');
  db.exec(readFileSync(join(root, 'worker/migrations/0001_problem_reports.sql'), 'utf8'));
  const add = db.prepare(`INSERT INTO problem_reports (id, created_at, tool_id, tool_version, core_version, build_hash,
    asset_versions_json, kind, page_path, note, note_has_url, inputs_json, outputs_json, warnings_json, display_json, dedupe_key,
    status, primary_source, fixed_in_version, resolved_at)
    VALUES (?, ?, ?, '1.0.0', '0.1.0', 'abc', '{}', 'wrong-result', '/x/', ?, ?, '[]', '[]', '[]', '{}', ?, ?, ?, ?, ?)`);
  const now = new Date().toISOString();
  add.run('a', now, 'aviation.altimetry.density-altitude', 'see www.example.com', 1, 'ka', 'open', null, null, null);
  add.run('b', now, 'aviation.altimetry.density-altitude', null, 0, 'kb', 'confirmed', 'ICAO Doc 7488/3', null, null);
  add.run('c', now, 'survey.cogo.inverse', null, 0, 'kc', 'fixed', 'NGS', '1.1.0', now);
  assert.equal(queries.length, 4);
  const results = queries.map((q) => db.prepare(q).all());
  assert.deepEqual(results.map((r) => r.length), [1, 1, 1, 1]);
  assert.equal(results[0][0].open_reports, 1);
  assert.equal(results[1][0].id, 'b');
  assert.equal(results[2][0].id, 'a');
  assert.equal(results[3][0].fixed_in_version, '1.1.0');
});
