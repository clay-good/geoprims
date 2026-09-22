// Formulas as MathML at build time (web/tool-docs 9.1): real expressions
// become MathML with their text kept as alttext; prose stays prose.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { toMathML } from '../src/lib/mathml.mjs';
import { disagreements, vectorFor } from '../src/lib/worked.mjs';

test('expressions become MathML: fractions stay inline, powers rise, subscripts sink, roots enclose', () => {
  const m = toMathML('h = sin²(Δφ/2) + cos φ₁ × cos φ₂ × sin²(Δλ/2)');
  assert.ok(m.startsWith('<math display="block" alttext="h = sin²(Δφ/2)'));
  assert.match(m, /<msup><mi>sin<\/mi><mn>2<\/mn><\/msup>/);
  assert.match(m, /<msub><mi>φ<\/mi><mrow><mn>1<\/mn><\/mrow><\/msub>/);
  assert.match(toMathML('d = 2 × R × asin(√h)'), /<msqrt><mi>h<\/mi><\/msqrt>/);
  assert.match(toMathML('n = 2^zoom'), /<msup><mn>2<\/mn><mi>zoom<\/mi><\/msup>/);
  assert.match(toMathML('λ₀ = 6° × zone − 183°'), /<mo>°<\/mo>/);
  assert.match(toMathML('A = |2A| / 2'), /<mo>\|<\/mo><mn>2<\/mn><mi>A<\/mi><mo>\|<\/mo>/, 'a number times a symbol, side by side');
});

test('prose, paths, and broken input stay as text', () => {
  assert.equal(toMathML('p = ((QNH/1013.25)^0.190284 − elevation × 6.8756e-6)^(1/0.190284) × 1013.25'), null, 'too long to set without wrapping');
  for (const f of ['M = the sum of weight × arm for every station', 'headwind = wind speed × cos θ', 'tile = zoom/x/y', 'the centre of the cell the point falls in', 'x = (1 + 2', '']) {
    assert.equal(toMathML(f), null, f);
  }
});

test('the example gate: a vector found by value, and a changed answer caught', () => {
  const vectors = [{ id: 'v001', input: { altimeter: '29.8 inHg', elevation: '5000 ft' }, expect: { 'result.da.value': 100, ok: true, 'meta.warnings.*.code': 'W' }, tolerance: { 'result.da.value': { abs: 1, rel: 0.01 } } }, { id: 'v002', supersededBy: 'v003', input: { a: 1 } }];
  const v = vectorFor({ elevation: '5000 ft', altimeter: '29.80 inHg' }, vectors);
  assert.equal(v.id, 'v001', '29.80 and 29.8 are the same value');
  assert.equal(vectorFor({ a: 1 }, vectors), null, 'a superseded vector does not count');
  const ok = { ok: true, result: { da: { value: 101.9 } }, meta: { warnings: [{ code: 'X' }, { code: 'W' }] } };
  assert.deepEqual(disagreements(ok, v), [], 'within abs 1 + 1% of 100');
  assert.equal(disagreements({ ...ok, result: { da: { value: 102.1 } } }, v).length, 1);
  assert.equal(disagreements({ ...ok, meta: { warnings: [] } }, v).length, 1);
});
