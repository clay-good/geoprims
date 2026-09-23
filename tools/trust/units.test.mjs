// The answer must not depend on whether the unit was written out. A field that
// takes a quantity accepts "5000 ft" and a bare 5000, reading the bare one in
// the unit it declares, so the two must come back with the same answer for
// every tool. A field whose schema says "number" and nothing else is a plain
// number that happens to carry a unit label -- decibels, which are logarithmic
// and do not convert -- and is left alone here, because its schema already
// tells a caller that a string is not allowed.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import { nodeHost } from '../../packages/runtime/src/node.mjs';

const root = join(import.meta.dirname, '../..');

/** Whether this field takes a written quantity, as against a labelled number. */
function takesAUnit(schema) {
  const type = schema.type;
  const allowsString = Array.isArray(type) ? type.includes('string') : type === 'string';
  return allowsString && !!schema['x-quantity'] && !!schema['x-unit'] && schema['x-unit'] !== '1';
}

test('a value written with its unit gives the same answer as the bare number', async () => {
  const host = nodeHost(join(root, 'dist/wasm'));
  const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
  const differ = [];
  let checked = 0;
  const dimensions = new Set();
  for (const t of catalog.tools) {
    const primary = t.examples?.find((e) => e.id === t['x-primary-example']);
    if (!primary) continue;
    const base = JSON.parse(await host.invoke(t.id, JSON.stringify(primary.input)));
    if (base.ok !== true) continue;
    for (const [name, schema] of Object.entries(t.inputs?.properties ?? {})) {
      const v = primary.input[name];
      if (typeof v !== 'number' || !takesAUnit(schema)) continue;
      dimensions.add(schema['x-quantity']);
      const unit = schema['x-unit'];
      const r = JSON.parse(await host.invoke(t.id, JSON.stringify({ ...primary.input, [name]: `${v} ${unit}` })));
      checked++;
      if (r.ok !== true) {
        differ.push(`${t.id}: ${name} = "${v} ${unit}" was refused: ${r.error?.message}`);
      } else if (JSON.stringify(r.result) !== JSON.stringify(base.result)) {
        differ.push(`${t.id}: ${name} = "${v} ${unit}" answers differently from ${v}`);
      }
    }
  }
  assert.ok(checked > 200, `only ${checked} quantities were reachable to test`);
  assert.ok(dimensions.size >= 8, `only ${dimensions.size} dimensions covered`);
  assert.deepEqual(differ, [], `${differ.length} of ${checked}:\n${differ.join('\n')}`);
});
