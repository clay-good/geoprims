// The practitioner vocabulary promised by add-navigation-and-geometry must
// remain an addressable catalog alias, not merely wording buried in prose.
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
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
  // 15 from add-navigation-and-geometry, plus the add-sampling-and-grids tools.
  assert.deepEqual(count('geometry'), { operations: 18, endpoints: 18 });
});

test('every registered navigation and geometry operation has derivation documentation', () => {
  const operations = catalog.tools.filter(
    (tool) => ['navigation', 'geometry'].includes(tool.domain) && tool.composedOf.length === 0,
  );
  const missing = operations
    .filter((tool) => !existsSync(join(root, 'docs/derivations', `${tool.id}.md`)))
    .map((tool) => tool.id);
  assert.deepEqual(missing, []);
});

test('every add-navigation-and-geometry operation is stable', () => {
  // The expansion wave's sampling tools start experimental (add-sampling-and-grids).
  const operations = catalog.tools.filter(
    (tool) => ['navigation', 'geometry'].includes(tool.domain) && tool.composedOf.length === 0 && tool.group !== 'sampling',
  );
  assert.ok(operations.length > 0);
  assert.deepEqual(
    operations.filter((tool) => tool.stability !== 'stable').map((tool) => tool.id),
    [],
  );
  assert.deepEqual(
    operations.filter((tool) => tool.vectorCount < 20).map((tool) => tool.id),
    [],
  );
});

test('the haversine versus ellipsoid explainer covers each method it compares', () => {
  const explainer = readFileSync(
    join(root, 'apps/web/src/content/learn/geodesic-vs-haversine.md'),
    'utf8',
  );
  for (const id of [
    'navigation.geodesic.haversine',
    'navigation.geodesic.inverse',
    'navigation.geodesic.spherical-inverse',
    'navigation.geodesic.vincenty-inverse',
    'navigation.rhumb.inverse',
  ]) {
    assert.match(explainer, new RegExp(`^  - ${id}$`, 'm'), `${id}: not listed by the explainer`);
  }
  assert.match(explainer, /^## Why the two differ$/m);
  assert.match(explainer, /^## Worked examples$/m);
});
