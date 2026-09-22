// add-practitioner-essentials "Removed tools": the Koch-chart takeoff estimate
// (it would be mistaken for aircraft data) and the angle-of-repose reference
// table (unclear sources, slope-design liability) stay out of the catalog.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';

const path = new URL('../../dist/catalog/v1.json', import.meta.url);
const built = existsSync(path) ? false : 'needs npm run build';

test('the removed tools are not in the catalog', { skip: built }, () => {
  const catalog = JSON.parse(readFileSync(path, 'utf8'));
  const ids = catalog.tools.map((t) => t.id);
  assert.deepEqual(ids.filter((id) => /koch|angle-of-repose/.test(id)), []);
  // No tool offers a generic takeoff or landing distance estimate: only the POH table.
  const generic = catalog.tools.filter((t) => /takeoff distance|landing distance/i.test(t.title));
  assert.deepEqual(generic.map((t) => t.id), []);
});
