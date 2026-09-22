// The answer's provenance (web/app-shell, "Result panel"): this result's
// versions, model, accuracy, reference data by name, notes, and citations.
import { join } from 'node:path';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { provenanceRows } from '../src/lib/provenance.js';
import { nodeHost } from '../../../packages/runtime/src/node.mjs';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const host = nodeHost(join(root, 'dist/wasm'));
const registry = JSON.parse(readFileSync(join(root, 'assets/registry.json'), 'utf8'));

test('a geoid result names its versions, its dataset, and what it cites', async () => {
  const r = JSON.parse(await host.invoke('geodesy.geoid.geoid-height', JSON.stringify({ lat: 40, lon: -105 })));
  const rows = Object.fromEntries(provenanceRows(r.meta, registry).map((x) => [x.label, x.value]));
  assert.equal(rows['Computed by'], `geodesy.geoid.geoid-height ${r.meta.toolVersion}, core ${r.meta.coreVersion}`);
  assert.match(rows['Reference data'], /EGM96 geoid, 15-minute grid, version 2009-08-29/);
  assert.doesNotMatch(rows['Reference data'], /egm96-15/, 'a dataset by name, not by id');
  assert.equal(rows.Notes, `${r.meta.warnings.length} shown with the answer`);
  assert.match(rows.Cites, /Lemoine/);
  assert.equal(rows.Model, r.meta.model);
});

test('before the registry loads, a dataset is still shown, and no meta means no rows', () => {
  const rows = provenanceRows({ tool: 't', toolVersion: '1', coreVersion: '2', assets: [{ id: 'x', version: 'v1' }] }, null);
  assert.equal(rows.find((r) => r.label === 'Reference data').value, 'version v1 of a reference dataset');
  assert.equal(rows.find((r) => r.label === 'Notes').value, 'None');
  assert.deepEqual(provenanceRows(null), []);
});
