// Old permalinks after a field is renamed or removed (3.5, migration hooks).
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { migrateState } from '../src/lib/migrate.mjs';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const table = JSON.parse(readFileSync(join(root, 'data/link-migrations.json'), 'utf8'));
const tool = { id: 'demo.group.op', inputs: { properties: { altitude: {}, temperature: {} } } };

test('a renamed field lands under its new name, and a removed one is named, not lost silently', () => {
  const migrations = { 'demo.group.op': [{ since: '1.1.0', rename: { alt: 'altitude' }, drop: ['old_flag'] }] };
  const out = migrateState(tool, { alt: '5000 ft', temperature: '30 degC', old_flag: 'yes', stray: 1 }, migrations);
  assert.deepEqual(out.inputs, { altitude: '5000 ft', temperature: '30 degC' });
  assert.deepEqual(out.renamed, [['alt', 'altitude']]);
  assert.deepEqual(out.dropped, ['stray']);
  // A link that already uses the new name keeps its value.
  assert.deepEqual(migrateState(tool, { alt: '1', altitude: '2' }, migrations).inputs, { altitude: '2' });
});

test('the migration table names only real tools and fields they now take', () => {
  for (const [id, steps] of Object.entries(table.tools)) {
    const t = catalog.tools.find((x) => x.id === id);
    assert.ok(t, `${id} is not a tool`);
    for (const s of steps) for (const to of Object.values(s.rename ?? {})) assert.ok(to in t.inputs.properties, `${id}: ${to} is not an input`);
  }
});

test('with no migration, every current link opens unchanged', () => {
  for (const t of catalog.tools.slice(0, 50)) {
    const ex = (t.examples.find((e) => e.id === t['x-primary-example']) ?? t.examples[0]).input;
    const out = migrateState(t, ex, table.tools);
    assert.deepEqual(out.inputs, ex, t.id);
    assert.deepEqual(out.dropped, []);
  }
});
