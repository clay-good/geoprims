// The aviation suite's 83 planned operations (add-aviation-suite design,
// "Tool inventory"), each held to the tool and fields that answer it, so
// the count in tasks.md 7.1 is checked rather than estimated.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const root = new URL('../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const { groups, endpoints } = JSON.parse(readFileSync(join(root, 'data/aviation-operations.json'), 'utf8'));
const design = readFileSync(join(root, 'openspec/changes/add-aviation-suite/design.md'), 'utf8');
const tools = new Map(catalog.tools.map((t) => [t.id, t]));

test('the inventory lists exactly the operations the design plans', () => {
  for (const [group, ops] of Object.entries(groups)) {
    const row = design.match(new RegExp(`^\\| \`${group}\` \\| (\\d+) \\| (.+) \\|$`, 'm'));
    assert.ok(row, `${group} is not in the design's inventory`);
    const planned = row[2].split(', ').map((s) => s.replace(/ \(.*\)$/, ''));
    assert.equal(planned.length, Number(row[1]), `${group}: the design's count`);
    assert.deepEqual(Object.keys(ops), planned, group);
  }
  assert.equal(Object.values(groups).reduce((n, ops) => n + Object.keys(ops).length, 0), 83);
});

test('every built operation names a real tool and the fields that carry it', () => {
  for (const [group, ops] of Object.entries(groups)) {
    for (const [op, entry] of Object.entries(ops)) {
      if (!Array.isArray(entry)) {
        assert.ok(entry.pending?.length > 20, `${group}/${op}: say why it is pending`);
        continue;
      }
      const [id, ...fields] = entry;
      const t = tools.get(id);
      assert.ok(t, `${group}/${op}: no tool ${id}`);
      assert.ok(fields.length, `${group}/${op}: name the fields`);
      for (const f of fields) {
        if (f.startsWith('visualization:')) {
          assert.ok(t.visualization.some((l) => l.kind === f.slice(14)), `${id} draws no ${f}`);
        } else {
          assert.ok(f in t.inputs.properties || f in t.outputs.properties, `${group}/${op}: ${id} has no field ${f}`);
        }
      }
    }
  }
});

test('the generated aviation endpoints are exactly the listed ones, each a narrowed parent', () => {
  const generated = catalog.tools.filter((t) => t.domain === 'aviation' && t.composedOf.length).map((t) => t.id);
  assert.deepEqual(generated.sort(), [...endpoints.built].sort());
  for (const id of endpoints.built) {
    const t = tools.get(id);
    const parent = tools.get(t.composedOf[0]);
    assert.ok(parent && parent.composedOf.length === 0, `${id}: parent ${t.composedOf[0]}`);
    for (const f of Object.keys(t.inputs.properties)) assert.ok(f in parent.inputs.properties, `${id}: ${f} is not a parent input`);
    assert.deepEqual(Object.keys(t.outputs.properties), Object.keys(parent.outputs.properties), id);
  }
});
