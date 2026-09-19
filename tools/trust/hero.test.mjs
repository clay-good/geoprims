// docs/launch/hero-tools.md must match the build: every tool id exists, and a
// row's Stable box is checked exactly when all of its tools are stable.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const root = new URL('../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));

test('the hero-tool checklist matches the catalog', () => {
  const byId = new Map(catalog.tools.map((t) => [t.id, t]));
  const md = readFileSync(join(root, 'docs/launch/hero-tools.md'), 'utf8');
  const rows = md.split('\n').filter((l) => /^\| [^|]+ \| [^|]+ \| `/.test(l));
  assert.ok(rows.length >= 30, `only ${rows.length} hero rows`);
  const problems = [];
  let stableRows = 0;
  for (const row of rows) {
    const cells = row.split('|').map((c) => c.trim());
    const ids = [...cells[3].matchAll(/`([^`]+)`/g)].map((m) => m[1]);
    const missing = ids.filter((id) => !byId.has(id));
    if (missing.length) problems.push(`${missing.join(', ')} is not in the catalog`);
    const stable = ids.every((id) => byId.get(id)?.stability === 'stable');
    const checked = cells[4] === '[x]';
    if (checked) stableRows++;
    if (stable !== checked) problems.push(`${ids.join(', ')}: Stable box says ${checked}, the build says ${stable}`);
  }
  const total = /^Stable: (\d+) of (\d+) rows/m.exec(md);
  assert.ok(total, 'needs a "Stable: N of M rows" line');
  if (+total[1] !== stableRows || +total[2] !== rows.length) problems.push(`the total should read ${stableRows} of ${rows.length}`);
  assert.deepEqual(problems, []);
});
