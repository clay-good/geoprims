// Survey-suite catalog progress and the practitioner search language promised
// by add-survey-suite. The 57-operation target stays in the spec until every
// planned operation is independently addressable.
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

const root = new URL('../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const survey = catalog.tools.filter((tool) => tool.domain === 'survey' && tool.composedOf.length === 0);
const byId = new Map(survey.map((tool) => [tool.id, tool]));

test('the direction parser is an addressable survey operation', () => {
  const tool = byId.get('survey.cogo.direction-parse');
  assert.ok(tool, 'survey.cogo.direction-parse is missing');
  for (const alias of ['bearing to azimuth', 'azimuth to bearing', 'quadrant bearing converter']) {
    assert.ok(tool.aliases.includes(alias), `direction parser is missing alias "${alias}"`);
  }
  assert.ok(existsSync(join(root, 'docs/derivations/survey.cogo.direction-parse.md')));
  assert.ok(tool.vectorCount >= 20, `direction parser has only ${tool.vectorCount} vectors`);
});

test('deflection angle is an addressable survey operation', () => {
  const tool = byId.get('survey.cogo.deflection-angle');
  assert.ok(tool, 'survey.cogo.deflection-angle is missing');
  for (const alias of ['deflection angle calculator', 'left right deflection', 'backsight foresight angle']) {
    assert.ok(tool.aliases.includes(alias), `deflection-angle is missing alias "${alias}"`);
  }
  assert.ok(existsSync(join(root, 'docs/derivations/survey.cogo.deflection-angle.md')));
  assert.ok(tool.vectorCount >= 20, `deflection-angle has only ${tool.vectorCount} vectors`);
});

test('radial sideshots are an addressable survey operation', () => {
  const tool = byId.get('survey.cogo.radial-sideshots');
  assert.ok(tool, 'survey.cogo.radial-sideshots is missing');
  for (const alias of ['radial survey calculator', 'multiple sideshots', 'total station sideshots']) {
    assert.ok(tool.aliases.includes(alias), `radial-sideshots is missing alias "${alias}"`);
  }
  assert.ok(existsSync(join(root, 'docs/derivations/survey.cogo.radial-sideshots.md')));
  assert.ok(tool.vectorCount >= 20, `radial-sideshots has only ${tool.vectorCount} vectors`);
});

test('each traverse adjustment method is an addressable survey operation', () => {
  const expected = new Map([
    ['survey.cogo.bowditch', ['Bowditch adjustment', 'compass rule']],
    ['survey.cogo.transit', ['transit rule', 'transit traverse adjustment']],
    ['survey.cogo.crandall', ['Crandall rule', 'bearing-held traverse adjustment']],
  ]);
  for (const [id, aliases] of expected) {
    const tool = byId.get(id);
    assert.ok(tool, `${id} is missing`);
    for (const alias of aliases) assert.ok(tool.aliases.includes(alias), `${id} is missing alias "${alias}"`);
    assert.ok(existsSync(join(root, `docs/derivations/${id}.md`)), `${id} needs a derivation note`);
    assert.ok(tool.vectorCount >= 20, `${id} has only ${tool.vectorCount} vectors`);
  }
  assert.equal(byId.get('survey.cogo.bowditch').stability, 'stable');
});

test('each intersection method is an addressable survey operation', () => {
  const expected = new Map([
    ['survey.cogo.bearing-bearing', ['bearing bearing intersection', 'direction direction intersection']],
    ['survey.cogo.bearing-distance', ['bearing distance intersection', 'line circle intersection']],
    ['survey.cogo.distance-distance', ['distance distance intersection', 'circle circle intersection']],
  ]);
  for (const [id, aliases] of expected) {
    const tool = byId.get(id);
    assert.ok(tool, `${id} is missing`);
    for (const alias of aliases) assert.ok(tool.aliases.includes(alias), `${id} is missing alias "${alias}"`);
    assert.ok(existsSync(join(root, `docs/derivations/${id}.md`)), `${id} needs a derivation note`);
    assert.ok(tool.vectorCount >= 20, `${id} has only ${tool.vectorCount} vectors`);
  }
  assert.equal(byId.get('survey.cogo.bearing-bearing').stability, 'stable');
});

test('the survey catalog does not lose registered operations', () => {
  assert.ok(survey.length >= 53, `survey operation count regressed to ${survey.length}`);
});
