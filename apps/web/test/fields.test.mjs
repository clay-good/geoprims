// Stepping a typed value (ux/mobile-and-field, Field mode). A field holds text
// that may carry a unit and may be written with either decimal separator, so a
// step button has to move the number and leave everything else alone.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { flipped, isSigned, stepLabel, stepped, stepsOf } from '../src/lib/fields.mjs';

test('a step keeps the unit the user typed', () => {
  assert.equal(stepped('5000 ft', 100), '5100 ft');
  assert.equal(stepped('29.92 inHg', 0.01), '29.93 inHg');
  assert.equal(stepped('29.92 inHg', -0.1), '29.82 inHg');
  assert.equal(stepped('145 kt', -10), '135 kt');
});

test('a step writes the result in the reader\u2019s number format', () => {
  assert.equal(stepped('29,92 inHg', 0.01, 'decimal-comma'), '29,93 inHg');
  assert.equal(stepped('1.250,5 m', 10, 'decimal-comma'), '1260,5 m');
});

test('a step reads a grouped number, and writes an ungrouped one', () => {
  assert.equal(stepped('1,250 ft', 100), '1350 ft');
});

test('a step does not leave floating-point noise behind', () => {
  assert.equal(stepped('29.92', 0.01), '29.93');
  assert.equal(stepped('0.1', 0.2), '0.3');
});

test('an empty field starts from the step', () => {
  assert.equal(stepped('', 10), '10');
  assert.equal(stepped('', -0.01), '-0.01');
});

test('a value that is not a number is left in front of the step', () => {
  assert.equal(stepped('KSFO', 1), '1 KSFO');
});

test('a negative value steps like any other', () => {
  assert.equal(stepped('-12 °C', 1), '-11 °C');
  assert.equal(stepped('-12 °C', -1), '-13 °C');
});

test('the four buttons run from the largest decrease to the largest increase', () => {
  assert.deepEqual(stepsOf({ 'x-step': { small: 0.01, large: 0.1 } }), [-0.1, -0.01, 0.01, 0.1]);
  assert.deepEqual(stepsOf({}), []);
  assert.deepEqual(['−10', '−1', '+1', '+10'], stepsOf({ 'x-step': { small: 1, large: 10 } }).map(stepLabel));
});

test('the sign toggle and the signed rule still hold', () => {
  assert.equal(flipped('30 °C'), '-30 °C');
  assert.equal(isSigned('lat', { 'x-quantity': 'angle' }), true);
  assert.equal(isSigned('radius', { 'x-quantity': 'length' }), false);
});
