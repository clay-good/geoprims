// docs/launch/hero-tools.md must match the build: every tool id exists, a
// row's Stable box is checked exactly when all of its tools are stable, and its
// "Shows its work" box exactly when all of them show their work, as a
// formula trace, a decoder's coded groups, or a list of rows the card tables.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { nodeHost } from '../../packages/runtime/src/node.mjs';
import { rowTables } from '../../apps/web/src/lib/rows.js';

const root = new URL('../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const host = nodeHost(join(root, 'dist/wasm'));

/** The tool ids that show their work for their own primary example. */
async function showWork() {
  const ids = new Set();
  for (const t of catalog.tools) {
    const ex = t.examples.find((e) => e.id === t['x-primary-example']) ?? t.examples[0];
    const input = { ...ex.input, options: { ...(ex.input.options ?? {}), explain: true } };
    const r = JSON.parse(await host.invoke(t.id, JSON.stringify(input)));
    // Three shapes count, because three shapes are shown: a formula trace, a
    // decoder's coded groups, and a list of rows the answer card tables.
    if (r.ok && (r.trace?.length || r.result?.groups?.length || rowTables(r).length)) ids.add(t.id);
  }
  return ids;
}

test('the hero-tool checklist matches the catalog', async () => {
  const showing = await showWork();
  const byId = new Map(catalog.tools.map((t) => [t.id, t]));
  const md = readFileSync(join(root, 'docs/launch/hero-tools.md'), 'utf8');
  const rows = md.split('\n').filter((l) => /^\| [^|]+ \| [^|]+ \| `/.test(l));
  assert.ok(rows.length >= 30, `only ${rows.length} hero rows`);
  const problems = [];
  let stableRows = 0;
  let workRows = 0;
  for (const row of rows) {
    const cells = row.split('|').map((c) => c.trim());
    const ids = [...cells[3].matchAll(/`([^`]+)`/g)].map((m) => m[1]);
    const missing = ids.filter((id) => !byId.has(id));
    if (missing.length) problems.push(`${missing.join(', ')} is not in the catalog`);
    const stable = ids.every((id) => byId.get(id)?.stability === 'stable');
    const checked = cells[4] === '[x]';
    if (checked) stableRows++;
    if (stable !== checked) problems.push(`${ids.join(', ')}: Stable box says ${checked}, the build says ${stable}`);
    const works = ids.length > 0 && ids.every((id) => showing.has(id));
    const worksChecked = cells[7] === '[x]';
    if (worksChecked) workRows++;
    if (works !== worksChecked) problems.push(`${ids.join(', ')}: "Shows its work" says ${worksChecked}, the build says ${works}`);
  }
  const total = /^Stable: (\d+) of (\d+) rows\. Shows its work: (\d+) of (\d+) rows/m.exec(md);
  assert.ok(total, 'needs a "Stable: N of M rows. Shows its work: N of M rows" line');
  if (+total[1] !== stableRows || +total[2] !== rows.length) problems.push(`the stable total should read ${stableRows} of ${rows.length}`);
  if (+total[3] !== workRows || +total[4] !== rows.length) problems.push(`the work total should read ${workRows} of ${rows.length}`);
  assert.deepEqual(problems, []);
});
