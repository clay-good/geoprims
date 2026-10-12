// The practitioner vocabulary promised by add-navigation-and-geometry must
// remain an addressable catalog alias, not merely wording buried in prose.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

const root = new URL('../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const byId = new Map(catalog.tools.map((tool) => [tool.id, tool]));

test('navigation tools expose the specified practitioner aliases', () => {
  const expected = {
    'navigation.geodesic.inverse': 'as the crow flies',
    'navigation.geodesic.spherical-inverse': 'great circle',
    'navigation.route.cross-track': 'XTE',
    'navigation.rhumb.inverse': 'loxodrome',
  };
  for (const [id, alias] of Object.entries(expected)) {
    assert.ok(byId.get(id)?.aliases.includes(alias), `${id}: missing alias "${alias}"`);
  }
});

test('navigation and geometry catalog progress is counted by the operation/endpoint contract', () => {
  const count = (domain) => {
    const tools = catalog.tools.filter((tool) => tool.domain === domain);
    return {
      operations: tools.filter((tool) => tool.composedOf.length === 0).length,
      endpoints: tools.length,
    };
  };
  assert.deepEqual(count('navigation'), { operations: 31, endpoints: 31 });
  assert.deepEqual(count('geometry'), { operations: 15, endpoints: 15 });
});
