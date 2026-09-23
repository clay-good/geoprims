// Golden vectors for indexing.s2.covering, over the same sixteen regions the
// containment fixture uses (tools/vectors/gen_s2_cover.py).
//
// A covering is not unique -- S2's own coverer and this one both produce valid
// coverings of the same region and they are different sets -- so the cell list
// here is this implementation's choice, pinned so a change in the refinement is
// visible. What makes the answer *right* rather than merely stable is checked
// elsewhere and independently: tests/s2_cover_parity.rs asks s2sphere for
// points inside each region and requires the covering to hold one of each
// point's own ancestors.
//
//   node tools/vectors/gen_s2_cover_vectors.mjs
import { readFileSync, appendFileSync } from 'node:fs';
import { join } from 'node:path';
import { nodeHost } from '../../packages/runtime/src/node.mjs';

const root = new URL('../..', import.meta.url).pathname;
const out = join(root, 'core/vectors/indexing.s2.covering.jsonl');
const fixture = join(root, 'core/crates/gp-indexing/tests/data/s2_cover.jsonl');
const SRC =
  'The covering this implementation produces, pinned so a change in the refinement is visible; ' +
  'containment is checked independently against s2sphere in tests/s2_cover_parity.rs';
const VER = '2026-09-23';

const existing = readFileSync(out, 'utf8').split('\n').filter(Boolean).map((l) => JSON.parse(l));
const start = Math.max(...existing.map((v) => Number(v.id.slice(1))));
if (start > 8) throw new Error(`${out} already carries ${start} vectors; this appends once`);

const host = nodeHost(join(root, 'dist/wasm'));
const rows = readFileSync(fixture, 'utf8').split('\n').filter((l) => l && !l.startsWith('#')).map((l) => JSON.parse(l));
let i = start;
for (const row of rows) {
  const [a, b, c, d] = row.params;
  const area =
    row.kind === 'rect'
      ? { south: `${a} deg`, north: `${b} deg`, west: `${c} deg`, east: `${d} deg` }
      : { lat: a, lon: b, radius: `${c} m` };
  const input = { ...area, min_level: row.min_level, max_level: row.max_level, max_cells: row.max_cells };
  const r = JSON.parse(await host.invoke('indexing.s2.covering', JSON.stringify(input)));
  if (!r.ok) throw new Error(`${JSON.stringify(input)} -> ${r.error.message}`);
  i += 1;
  const expect = {
    'result.count': r.result.count,
    'result.coarsest_level': r.result.coarsest_level,
    'result.finest_level': r.result.finest_level,
    'result.cells.0.cell': r.result.cells[0].cell,
    [`result.cells.${r.result.cells.length - 1}.cell`]: r.result.cells.at(-1).cell,
    ok: true,
  };
  const tolerance = {
    'result.count': { abs: 0 },
    'result.coarsest_level': { abs: 0 },
    'result.finest_level': { abs: 0 },
  };
  appendFileSync(out, JSON.stringify({ id: `v${String(i).padStart(3, '0')}`, input, expect, source: SRC, sourceVersion: VER, tolerance }) + '\n');
}
console.log(`appended ${rows.length} vectors to indexing.s2.covering.jsonl (now ${existing.length + rows.length})`);
