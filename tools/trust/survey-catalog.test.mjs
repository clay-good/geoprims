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

test('azimuth and bearing conversion is an addressable survey operation', () => {
  const tool = byId.get('survey.cogo.azimuth-bearing');
  assert.ok(tool, 'survey.cogo.azimuth-bearing is missing');
  for (const alias of ['azimuth bearing converter', 'bearing azimuth conversion', 'azimuth to quadrant bearing']) {
    assert.ok(tool.aliases.includes(alias), `azimuth-bearing is missing alias "${alias}"`);
  }
  assert.ok(existsSync(join(root, 'docs/derivations/survey.cogo.azimuth-bearing.md')));
  assert.ok(tool.vectorCount >= 20, `azimuth-bearing has only ${tool.vectorCount} vectors`);
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

test('each station-offset direction is an addressable survey operation', () => {
  const expected = new Map([
    ['survey.cogo.perpendicular-foot', ['foot of perpendicular', 'perpendicular offset']],
    ['survey.cogo.point-from-station-offset', ['point from station and offset', 'station offset to coordinates']],
  ]);
  for (const [id, aliases] of expected) {
    const tool = byId.get(id);
    assert.ok(tool, `${id} is missing`);
    for (const alias of aliases) assert.ok(tool.aliases.includes(alias), `${id} is missing alias "${alias}"`);
    assert.ok(existsSync(join(root, `docs/derivations/${id}.md`)), `${id} needs a derivation note`);
    assert.ok(tool.vectorCount >= 20, `${id} has only ${tool.vectorCount} vectors`);
  }
});

test('each combined-factor step and direction is an addressable survey operation', () => {
  const expected = new Map([
    ['survey.reduction.elevation-factor', ['elevation factor calculator', 'sea level factor']],
    ['survey.reduction.grid-to-ground', ['grid to ground', 'grid to ground distance']],
    ['survey.reduction.ground-to-grid', ['ground to grid', 'ground to grid distance']],
  ]);
  for (const [id, aliases] of expected) {
    const tool = byId.get(id);
    assert.ok(tool, `${id} is missing`);
    for (const alias of aliases) assert.ok(tool.aliases.includes(alias), `${id} is missing alias "${alias}"`);
    assert.ok(existsSync(join(root, `docs/derivations/${id}.md`)), `${id} needs a derivation note`);
    assert.ok(tool.vectorCount >= 20, `${id} has only ${tool.vectorCount} vectors`);
  }
});

test('each offset-shot method is an addressable survey operation', () => {
  const expected = new Map([
    ['survey.reduction.distance-offset', ['distance offset', 'taped offset shot']],
    ['survey.reduction.angle-offset', ['angle offset', 'tree center shot']],
  ]);
  for (const [id, aliases] of expected) {
    const tool = byId.get(id);
    assert.ok(tool, `${id} is missing`);
    for (const alias of aliases) assert.ok(tool.aliases.includes(alias), `${id} is missing alias "${alias}"`);
    assert.ok(existsSync(join(root, `docs/derivations/${id}.md`)), `${id} needs a derivation note`);
    assert.ok(tool.vectorCount >= 20, `${id} has only ${tool.vectorCount} vectors`);
  }
});

test('the two-face mean and the level adjustment are addressable survey operations', () => {
  const expected = new Map([
    ['survey.reduction.two-face-mean', ['two face mean', 'vertical index error']],
    ['survey.reduction.level-adjust', ['level adjustment', 'level loop adjustment']],
  ]);
  for (const [id, aliases] of expected) {
    const tool = byId.get(id);
    assert.ok(tool, `${id} is missing`);
    for (const alias of aliases) assert.ok(tool.aliases.includes(alias), `${id} is missing alias "${alias}"`);
    assert.ok(existsSync(join(root, `docs/derivations/${id}.md`)), `${id} needs a derivation note`);
    assert.ok(tool.vectorCount >= 20, `${id} has only ${tool.vectorCount} vectors`);
  }
});

test('each earthwork step named in the inventory is an addressable survey operation', () => {
  const expected = new Map([
    ['survey.earthwork.rise-run', ['rise over run calculator', 'slope length calculator']],
    ['survey.earthwork.haul-loads', ['haul loads calculator', 'how many truck loads']],
    ['survey.earthwork.four-point-average', ['four point method', 'average depth grid volume']],
  ]);
  for (const [id, aliases] of expected) {
    const tool = byId.get(id);
    assert.ok(tool, `${id} is missing`);
    for (const alias of aliases) assert.ok(tool.aliases.includes(alias), `${id} is missing alias "${alias}"`);
    assert.ok(existsSync(join(root, `docs/derivations/${id}.md`)), `${id} needs a derivation note`);
    assert.ok(tool.vectorCount >= 20, `${id} has only ${tool.vectorCount} vectors`);
  }
});

test('each curve operation named in the inventory is an addressable survey operation', () => {
  const expected = new Map([
    ['survey.curves.degree-of-curve', ['degree of curve calculator', 'arc definition degree of curve']],
    ['survey.curves.curve-stationing', ['curve stationing calculator', 'PC and PT stations']],
    ['survey.curves.spiral-curve-spiral', ['spiral curve spiral', 'TS SC CS ST stations']],
    ['survey.curves.vertical-curve-turning-point', ['vertical curve turning point', 'low point of a sag curve']],
    ['survey.curves.vertical-curve-elevations', ['vertical curve elevations', 'vertical curve grade sheet']],
  ]);
  for (const [id, aliases] of expected) {
    const tool = byId.get(id);
    assert.ok(tool, `${id} is missing`);
    for (const alias of aliases) assert.ok(tool.aliases.includes(alias), `${id} is missing alias "${alias}"`);
    assert.ok(existsSync(join(root, `docs/derivations/${id}.md`)), `${id} needs a derivation note`);
    assert.ok(tool.vectorCount >= 20, `${id} has only ${tool.vectorCount} vectors`);
  }
});

// The 57-operation design inventory of add-survey-suite. Eight operations
// were built under a shorter ID than the inventory's name; the pairs are
// listed so the count stays honest until the spec or the IDs are reconciled.
const BUILT_AS = new Map([
  ['reduction.slope-reduction', 'reduction.slope'],
  ['reduction.edm-atmospheric', 'reduction.edm-correction'],
  ['earthwork.grade-convert', 'earthwork.grade'],
  ['earthwork.slope-staking', 'earthwork.slope-stake'],
  ['earthwork.stockpile-tin', 'earthwork.stockpile'],
  ['earthwork.solid-volumes', 'earthwork.solid-volume'],
  ['earthwork.profile-slope', 'earthwork.profile-grades'],
  ['curves.sight-distance-length', 'curves.sight-distance'],
]);
const INVENTORY = {
  cogo: 'direction-parse inverse forward radial-sideshots traverse-closure angular-closure bowditch transit crandall least-squares-2d area-by-coordinates bearing-bearing bearing-distance distance-distance resection station-offset point-from-station-offset perpendicular-foot azimuth-bearing deflection-angle',
  reduction: 'slope-reduction two-face-mean curvature-refraction edm-atmospheric elevation-factor combined-factor grid-to-ground ground-to-grid level-run level-adjust stadia inaccessible-height distance-offset angle-offset',
  earthwork: 'average-end-area prismoidal section-area borrow-pit four-point-average shrink-swell haul-loads grade-convert rise-run slope-staking stockpile-tin solid-volumes profile-slope',
  curves: 'circular-curve degree-of-curve curve-stationing curve-layout spiral spiral-curve-spiral vertical-curve vertical-curve-turning-point vertical-curve-elevations sight-distance-length',
};

test('every operation in the 57-item design inventory is built', () => {
  const planned = Object.entries(INVENTORY).flatMap(([group, names]) => names.split(' ').map((name) => `${group}.${name}`));
  assert.equal(planned.length, 57);
  const missing = planned.filter((name) => !byId.has(`survey.${BUILT_AS.get(name) ?? name}`));
  assert.deepEqual(missing, []);
});

test('the survey catalog does not lose registered operations', () => {
  assert.ok(survey.length >= 71, `survey operation count regressed to ${survey.length}`);
});
