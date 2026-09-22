// Personal ranking in the palette: pinned and recent tools rise a little, the
// first result never moves, and filled or detected answers keep their place.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { personalize, PIN_LIFT } from '../src/lib/boost.js';

const tools = (...ids) => ids.map((id) => ({ kind: 'tool', id }));
const ids = (rs) => rs.map((r) => r.id);

test('a pinned tool climbs up to its lift, but never past the first result', () => {
  const r = tools('a', 'b', 'c', 'd', 'e', 'f');
  assert.deepEqual(ids(personalize(r, { pinned: ['f'] })), ['a', 'b', 'f', 'c', 'd', 'e'], 'sixth rises three places');
  assert.deepEqual(ids(personalize(r, { pinned: ['c'] })), ['a', 'c', 'b', 'd', 'e', 'f']);
  assert.deepEqual(ids(personalize(r, { pinned: ['b'] })), ids(r), 'second place is already as high as a nudge goes');
  assert.equal(PIN_LIFT, 3);
});

test('recent tools rise less than pinned ones, and pinned stay ahead', () => {
  const r = tools('a', 'b', 'c', 'd', 'e');
  assert.deepEqual(ids(personalize(r, { recent: ['e'] })), ['a', 'b', 'e', 'c', 'd'], 'fifth rises two places');
  // d (pinned) rises to second; e (recent) then rises two, but not past d.
  assert.deepEqual(ids(personalize(r, { pinned: ['d'], recent: ['e'] })), ['a', 'd', 'e', 'b', 'c']);
  assert.deepEqual(ids(personalize(tools('a', 'b', 'c'), { pinned: ['b'], recent: ['c'] })), ['a', 'b', 'c'], 'a recent tool never passes a pinned one');
});

test('the same state always gives the same order, and answers keep their places', () => {
  const r = [{ kind: 'filled', id: 'x' }, { kind: 'detected', id: 'y' }, ...tools('a', 'b', 'c', 'd')];
  const once = personalize(r, { pinned: ['d'] });
  assert.deepEqual(ids(once), ['x', 'y', 'a', 'd', 'b', 'c']);
  assert.deepEqual(ids(personalize(r, { pinned: ['d'] })), ids(once));
  assert.deepEqual(ids(personalize(r)), ids(r), 'nothing pinned or recent: the core order');
  assert.deepEqual(personalize([], { pinned: ['a'] }), []);
});
