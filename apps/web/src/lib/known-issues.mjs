// Known issues (feedback/triage-and-corrections): the curated file, validated
// at build time, and the open confirmed entry for a tool, if any.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const root = join(process.cwd(), '../..');
const STATUSES = ['investigating', 'confirmed', 'fixed'];
const FIELDS = ['id', 'tool', 'summary', 'description', 'affected', 'workaround', 'status', 'since'];

export const issues = JSON.parse(readFileSync(join(root, 'data/known-issues.json'), 'utf8')).issues;
for (const i of issues) {
  for (const f of FIELDS) if (!i[f]) throw new Error(`known-issues.json: ${i.id ?? '?'} needs ${f}`);
  if (!STATUSES.includes(i.status)) throw new Error(`known-issues.json: ${i.id} has status ${i.status}`);
  if (i.status === 'fixed' && !(i.fixedIn && i.fixedOn)) throw new Error(`known-issues.json: ${i.id} is fixed but lacks fixedIn and fixedOn`);
}

/** The confirmed, unfixed issue for a tool: the page banner. */
export const openIssue = (toolId) => issues.find((i) => i.tool === toolId && i.status === 'confirmed');
