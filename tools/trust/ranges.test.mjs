// A declared range is a promise, and a promise is only kept if it is checked.
// The core checks one when the tool reads the field, so a field a tool reads
// only down one branch is never checked at all: survey.earthwork.slope-stake
// took a fill slope of 150 and of -0.99, both outside the 0.01 to 100 it
// declares, because its worked example is a cut section and the fill slope was
// never read. This feeds every tool's own worked example back to it with one
// number moved outside its range, and expects to be told no.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import { nodeHost } from '../../packages/runtime/src/node.mjs';

const root = join(import.meta.dirname, '../..');

/** A value outside [min, max] on the given side, or null if that side is open. */
function outside(schema, side) {
  const bound = side === 'above' ? schema.maximum : schema.minimum;
  if (typeof bound !== 'number') return null;
  const step = Math.max(1, Math.abs(bound) * 0.5);
  const v = side === 'above' ? bound + step : bound - step;
  return Number.isFinite(v) ? v : null;
}

test('every declared range is enforced on the value that is given', async () => {
  const host = nodeHost(join(root, 'dist/wasm'));
  const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
  const accepted = [];
  let checked = 0;
  for (const t of catalog.tools) {
    const primary = t.examples?.find((e) => e.id === t['x-primary-example']);
    if (!primary) continue;
    for (const [name, schema] of Object.entries(t.inputs?.properties ?? {})) {
      // Only the numbers the example itself gives: anything else would be
      // testing a field the tool was not asked about.
      if (typeof primary.input[name] !== 'number') continue;
      for (const side of ['above', 'below']) {
        const bad = outside(schema, side);
        if (bad === null) continue;
        const args = { ...primary.input, [name]: bad };
        const r = JSON.parse(await host.invoke(t.id, JSON.stringify(args)));
        checked++;
        if (r.ok === true) {
          accepted.push(
            `${t.id}: ${name} = ${bad} is ${side} its range of ${schema.minimum} to ${schema.maximum}, and was answered anyway`,
          );
        }
      }
    }
  }
  assert.ok(checked > 150, `only ${checked} ranges were reachable to test`);
  assert.deepEqual(accepted, [], `${accepted.length} of ${checked}:\n${accepted.join('\n')}`);
});
