// Every golden vector field is within its domain's tolerance ceiling, or its
// source is justified in data/tolerance-ceilings.json; no justification is stale.
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { nodeHost } from '../../packages/runtime/src/node.mjs';
import { bound, overCeiling } from './tolerance.mjs';

const root = new URL('../..', import.meta.url).pathname;
const data = JSON.parse(readFileSync(join(root, 'data/tolerance-ceilings.json'), 'utf8'));
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const host = nodeHost(join(root, 'dist/wasm'));
const domains = new Set(Object.keys(data.ceilings));

/** Display decimals of an output field at a result path, through lists. */
function decimals(tool, path) {
  let schema = { properties: tool.outputs.properties };
  for (const k of path.replace(/^result\./, '').replace(/\.value$/, '').split('.')) {
    if (/^\d+$/.test(k)) { schema = schema.items ?? schema; continue; }
    schema = schema.properties?.[k] ?? schema.items?.properties?.[k];
    if (!schema) return undefined;
  }
  return schema['x-display-precision']?.decimals;
}

test('golden vectors stay within their domain tolerance ceilings, or say why not', async () => {
  const used = new Set();
  const problems = [];
  let checked = 0;
  for (const file of readdirSync(join(root, 'core/vectors')).filter((f) => f.endsWith('.jsonl'))) {
    const id = file.slice(0, -'.jsonl'.length);
    const domain = id.split('.')[0];
    if (!domains.has(domain)) continue;
    const tool = catalog.tools.find((t) => t.id === id);
    for (const line of readFileSync(join(root, 'core/vectors', file), 'utf8').split('\n').filter(Boolean)) {
      const v = JSON.parse(line);
      if (v.supersededBy) continue;
      let got = null;
      for (const [path, want] of Object.entries(v.expect)) {
        if (typeof want !== 'number' || path.includes('.*.')) continue;
        checked++;
        const b = bound(v.tolerance?.[path], want);
        if (b === 0) continue;
        got ??= JSON.parse(await host.invoke(id, JSON.stringify(v.input)));
        const unit = path.endsWith('.value') ? path.slice(0, -'.value'.length).split('.').reduce((o, k) => o?.[k], got)?.unit : undefined;
        const why = overCeiling(domain, want, b, unit, tool && decimals(tool, path), data.ceilings);
        if (!why) continue;
        const ok = data.justified.findIndex((j) => j.tool === id && (v.source ?? '').startsWith(j.source));
        if (ok < 0) problems.push(`${id} ${v.id} ${path} allows ${why}, past the ${domain} ceiling, with no justification for "${(v.source ?? '').slice(0, 60)}"`);
        else used.add(ok);
      }
    }
  }
  assert.deepEqual(problems, []);
  const stale = data.justified.filter((_, i) => !used.has(i)).map((j) => `${j.tool}: "${j.source}" justifies nothing`);
  assert.deepEqual(stale, []);
  assert.ok(checked > 5_000, `${checked} fields checked`);
});

test('every justification gives a reason a reader can check', () => {
  for (const j of data.justified) {
    assert.ok(j.tool && j.source && j.reason, JSON.stringify(j));
    assert.ok(j.reason.split(/\s+/).length >= 6, `${j.tool}: "${j.reason}" is too short to be a reason`);
  }
});
