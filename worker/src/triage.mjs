// Triage status transitions and queries (feedback/triage-and-corrections, and
// design "Status workflow": open → triaged → confirmed → fixed, plus the
// terminal wont_fix, duplicate, and not_a_bug, each needing a resolution note;
// confirmed needs the primary source that settles it, and fixed the version
// that fixes it). Pure: the CLI in worker/scripts/triage.mjs prints or runs
// the SQL these return, and the tests run it against the real migration.

export const STATUSES = ['open', 'triaged', 'confirmed', 'fixed', 'wont_fix', 'duplicate', 'not_a_bug'];
export const TERMINAL = ['fixed', 'wont_fix', 'duplicate', 'not_a_bug'];

/** Where each status may go next. */
export const NEXT = {
  open: ['triaged', 'wont_fix', 'duplicate', 'not_a_bug'],
  triaged: ['confirmed', 'wont_fix', 'duplicate', 'not_a_bug'],
  confirmed: ['fixed', 'wont_fix'],
  fixed: [],
  wont_fix: [],
  duplicate: [],
  not_a_bug: [],
};

const TEXT_MAX = 1000;
const VERSION = /^\d+\.\d+\.\d+$/;

/**
 * The UPDATE that moves report `id` from `from` to `to`, or an Error saying
 * why the move is not allowed. `fields` may carry `primarySource`,
 * `resolutionNote`, and `fixedInVersion`; `now` is an ISO timestamp.
 */
export function transition({ id, from, to, fields = {}, now }) {
  const { primarySource, resolutionNote, fixedInVersion } = fields;
  const fail = (why) => new Error(`${id}: cannot move ${from} → ${to}: ${why}`);
  if (!STATUSES.includes(to)) return fail(`"${to}" is not a status (${STATUSES.join(', ')})`);
  if (!NEXT[from]?.includes(to)) return fail(`from ${from} the next status is one of: ${NEXT[from]?.join(', ') || 'none (it is final)'}`);
  const text = (v) => typeof v === 'string' && v.trim().length > 0 && v.length <= TEXT_MAX;
  if (to === 'confirmed' && !text(primarySource)) {
    return fail('a confirmed defect records the primary source that settles it (standard, reference implementation, or published worked example)');
  }
  if (to === 'fixed' && !(typeof fixedInVersion === 'string' && VERSION.test(fixedInVersion))) {
    return fail('a fix records the tool version that carries it, like 1.1.0');
  }
  if (['wont_fix', 'duplicate', 'not_a_bug'].includes(to) && !text(resolutionNote)) {
    return fail('a closed report records a resolution note');
  }
  const set = ['status = ?'];
  const params = [to];
  if (to === 'confirmed') set.push('primary_source = ?'), params.push(primarySource.trim());
  if (to === 'fixed') set.push('fixed_in_version = ?'), params.push(fixedInVersion);
  if (text(resolutionNote)) set.push('resolution_note = ?'), params.push(resolutionNote.trim());
  if (TERMINAL.includes(to)) set.push('resolved_at = ?'), params.push(now);
  // The WHERE on the old status makes a stale or concurrent move change nothing.
  return { sql: `UPDATE problem_reports SET ${set.join(', ')} WHERE id = ? AND status = ?`, params: [...params, id, from] };
}

/**
 * Open reports for triage: wrong results first, oldest first, with a flag on
 * wrong results older than 24 hours and anything older than 72 hours.
 */
export const TRIAGE_QUERY = `SELECT id, created_at, kind, tool_id, tool_version, page_path, note, note_has_url,
  CASE WHEN kind = 'wrong-result' AND created_at < ? THEN 1
       WHEN created_at < ? THEN 1 ELSE 0 END AS overdue
FROM problem_reports
WHERE status = 'open'
ORDER BY CASE WHEN kind = 'wrong-result' THEN 0 ELSE 1 END, created_at`;

/** The two cutoffs TRIAGE_QUERY binds: 24 and 72 hours before `now`. */
export function triageCutoffs(now) {
  const t = Date.parse(now);
  return [new Date(t - 24 * 3_600_000).toISOString(), new Date(t - 72 * 3_600_000).toISOString()];
}

/** One report with everything a reproduction needs. */
export const REPORT_QUERY = `SELECT id, created_at, tool_id, tool_version, core_version, build_hash, asset_versions_json,
  kind, page_path, inputs_json, outputs_json, warnings_json, note, status, primary_source, fixed_in_version
FROM problem_reports WHERE id = ?`;

/** Counts by status for the monthly /quality summary and the runbook. */
export const STATUS_COUNTS = `SELECT status, kind, COUNT(*) AS n FROM problem_reports GROUP BY status, kind ORDER BY status, kind`;
