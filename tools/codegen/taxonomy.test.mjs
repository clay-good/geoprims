import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import { taxonomyProblems } from './taxonomy.mjs';

const root = new URL('../..', import.meta.url).pathname;
const taxonomy = JSON.parse(readFileSync(join(root, 'data/taxonomy.json'), 'utf8'));
const tools = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8')).tools;

test('every built tool belongs to exactly one declared group', () => {
  assert.deepEqual(taxonomyProblems(tools, taxonomy), []);
  assert.ok(tools.length > 200);
});

test('the gate catches missing, duplicate, and contradictory mappings', () => {
  const sample = { id: 'geodesy.utm.forward', domain: 'geodesy', group: 'utm' };
  assert.match(taxonomyProblems([sample, sample], taxonomy).join('; '), /id appears twice/);
  assert.match(taxonomyProblems([{ ...sample, group: 'ups' }], taxonomy).join('; '), /disagrees with the id/);
  assert.match(taxonomyProblems([{ ...sample, id: 'geodesy.missing.forward' }], taxonomy).join('; '), /belongs to 0 groups/);
  const duplicate = structuredClone(taxonomy);
  duplicate.domains.geodesy.groups.push('utm');
  assert.match(taxonomyProblems([sample], duplicate).join('; '), /declared twice/);
  assert.match(taxonomyProblems([sample], duplicate).join('; '), /belongs to 2 groups/);
  const missing = structuredClone(taxonomy);
  delete missing.domains.raster;
  assert.match(taxonomyProblems([sample], missing).join('; '), /missing domain raster/);
});
