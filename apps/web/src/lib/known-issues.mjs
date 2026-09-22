// Known issues (feedback/triage-and-corrections): the curated file, validated
// at build time, and the open confirmed entry for a tool, if any.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const root = join(process.cwd(), '../..');
const STATUSES = ['investigating', 'confirmed', 'fixed'];
const FIELDS = ['id', 'tool', 'summary', 'description', 'affected', 'workaround', 'status', 'since'];

/** Throws on the first entry missing a field, with an unknown status, or fixed without its version and date. */
export function validate(list) {
  for (const i of list) {
    for (const f of FIELDS) if (!i[f]) throw new Error(`known-issues.json: ${i.id ?? '?'} needs ${f}`);
    if (!STATUSES.includes(i.status)) throw new Error(`known-issues.json: ${i.id} has status ${i.status}`);
    if (i.status === 'fixed' && !(i.fixedIn && i.fixedOn)) throw new Error(`known-issues.json: ${i.id} is fixed but lacks fixedIn and fixedOn`);
  }
  return list;
}

/** The confirmed, unfixed issue for a tool in `list`: the page banner. */
export const openIssueIn = (list, toolId) => list.find((i) => i.tool === toolId && i.status === 'confirmed');

export const issues = validate(JSON.parse(readFileSync(join(root, 'data/known-issues.json'), 'utf8')).issues);
export const openIssue = (toolId) => openIssueIn(issues, toolId);
