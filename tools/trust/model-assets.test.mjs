// Time-bounded models (platform/data-assets, "Time-bounded model validity"):
// a tool that uses a model with a validity window declares it, cites the
// model's ledger row (so its pages carry the expiring and expired notices),
// and refuses a date past the window with OUT_OF_DOMAIN naming it.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { nodeHost } from '../../packages/runtime/src/node.mjs';
import { readLedger, rowFor } from './ledger.mjs';

const root = new URL('../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const registry = JSON.parse(readFileSync(join(root, 'apps/web/public/assets/registry.json'), 'utf8'));
const ledger = readLedger(root);
const host = nodeHost(join(root, 'dist/wasm'));
const primary = (t) => t.examples.find((e) => e.id === t['x-primary-example']) ?? t.examples[0];
const windowed = registry.assets.filter((a) => a.validTo);

/**
 * Data compiled into the core that the asset registry does not list yet: the
 * leap-second table and the time-zone rules (add-practitioner-essentials 1.2
 * and 1.3 package them). The list may shrink, never grow.
 */
const UNREGISTERED = new Set(['leap-seconds', 'tzdb']);

/** The ledger row that dates a registry asset: its match term appears in the asset's title. */
const ledgerRowOf = (asset) => ledger.find((r) => r.validTo && r.matchTerms.some((m) => asset.title.includes(m)));

test('every dated model has a dated ledger row', () => {
  assert.ok(windowed.length >= 2, 'no dated models in the registry');
  for (const a of windowed) assert.ok(ledgerRowOf(a), `${a.id} has no ledger row with a validTo`);
});

test('a tool declares every asset its results use, and cites the dated ones', async () => {
  const problems = [];
  const seen = new Set();
  for (const t of catalog.tools) {
    const ex = primary(t);
    if (!ex) continue;
    const r = JSON.parse(await host.invoke(t.id, JSON.stringify(ex.input)));
    const used = (r.meta?.assets ?? []).map((a) => a.id);
    for (const id of used) {
      if (UNREGISTERED.has(id)) {
        seen.add(id);
        continue;
      }
      if (!t.assets.includes(id)) problems.push(`${t.id} uses ${id} but does not declare it`);
    }
    for (const id of t.assets) {
      const a = windowed.find((x) => x.id === id);
      if (!a) continue;
      const row = ledgerRowOf(a);
      if (!t.references.some((ref) => rowFor(ledger, ref)?.id === row.id)) {
        problems.push(`${t.id} uses ${id} but cites no "${row.matchTerms[0]}" source, so its page cannot warn when the model runs out`);
      }
    }
  }
  assert.deepEqual(problems, []);
  // An exception that is registered now, or no longer used, leaves the list.
  const registered = new Set(registry.assets.map((a) => a.id));
  assert.deepEqual([...UNREGISTERED].filter((id) => registered.has(id) || !seen.has(id)), [], 'remove it from UNREGISTERED');
});

test('a date past a model window is refused, naming the window', async () => {
  let checked = 0;
  const problems = [];
  for (const t of catalog.tools) {
    const models = t.assets.map((id) => windowed.find((a) => a.id === id)).filter(Boolean);
    if (!models.length || !('date' in t.inputs.properties)) continue;
    // An example that already gives a date has the place and model inputs the tool needs.
    const ex = t.examples.find((e) => 'date' in e.input);
    if (!ex) continue;
    // Half a year after the earliest window closes (WMM2025 and IGRF-14: 2030.0).
    const end = Math.min(...models.map((a) => Number(a.validTo)));
    const after = `${Math.floor(end)}-07-01`;
    const r = JSON.parse(await host.invoke(t.id, JSON.stringify({ ...ex.input, date: after })));
    checked += 1;
    const extrapolated = (r.meta?.warnings ?? []).some((w) => w.code === 'MODEL_EXTRAPOLATED');
    if (r.ok && !extrapolated) {
      problems.push(`${t.id} answered for ${after}, past the model window, without MODEL_EXTRAPOLATED`);
    } else if (!r.ok && (r.error.code !== 'OUT_OF_DOMAIN' || !String(r.error.message).includes(String(Math.floor(end))))) {
      problems.push(`${t.id} refused ${after} with ${r.error.code}: ${r.error.message}`);
    }
  }
  assert.deepEqual(problems, []);
  assert.ok(checked >= 4, `only ${checked} dated-model tools checked`);
});
