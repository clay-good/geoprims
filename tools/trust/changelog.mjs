// The changelog (trust/proof-display "/changelog", verification "Regression
// history"): data/changelog.json, and the gate tying it to the vectors. Every
// superseded golden vector is a published result that moved, so it must be
// listed in a result-change entry.
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

export const KINDS = { 'result-change': 'Result change', added: 'Added', fixed: 'Fixed', changed: 'Changed' };

export const readChangelog = (root) => JSON.parse(readFileSync(join(root, 'data/changelog.json'), 'utf8'));

/** "tool vNNN" for every superseded vector in core/vectors. */
export function supersededVectors(root) {
  const dir = join(root, 'core/vectors');
  const out = [];
  for (const f of readdirSync(dir).filter((x) => x.endsWith('.jsonl'))) {
    for (const line of readFileSync(join(dir, f), 'utf8').split('\n').filter(Boolean)) {
      const v = JSON.parse(line);
      if (v.supersededBy) out.push(`${f.slice(0, -'.jsonl'.length)} ${v.id}`);
    }
  }
  return out;
}

export function checkChangelog(changelog, { catalog, superseded, today }) {
  const problems = [];
  const ids = new Set(catalog.tools.map((t) => t.id));
  const listed = new Set();
  let prev = '9999-12-31';
  for (const [i, e] of changelog.entries.entries()) {
    const at = `entry ${i + 1} (${e.date})`;
    if (!KINDS[e.kind]) problems.push(`${at}: unknown kind ${e.kind}`);
    if (!/^\d{4}-\d{2}-\d{2}$/.test(e.date ?? '')) problems.push(`${at}: date must be YYYY-MM-DD`);
    else if (e.date > today) problems.push(`${at}: dated in the future`);
    if (e.date > prev) problems.push(`${at}: entries must be newest first`);
    prev = e.date;
    if (!e.summary?.trim()) problems.push(`${at}: needs a summary`);
    for (const t of e.tools ?? []) if (!ids.has(t)) problems.push(`${at}: unknown tool ${t}`);
    for (const v of e.vectors ?? []) {
      if (e.kind !== 'result-change') problems.push(`${at}: only result changes list vectors`);
      if (!e.tools.includes(v.split(' ')[0])) problems.push(`${at}: vector ${v} belongs to a tool the entry does not name`);
      listed.add(v);
    }
  }
  for (const v of superseded) if (!listed.has(v)) problems.push(`superseded vector ${v} has no result-change entry`);
  for (const v of listed) if (!superseded.includes(v)) problems.push(`${v} is listed but not superseded`);
  return problems;
}

/** Entries that name a tool. */
export const entriesFor = (changelog, id) => changelog.entries.filter((e) => e.tools.includes(id));
