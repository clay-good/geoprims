// The sticky answer above the keyboard (ux/mobile-and-field). A phone keyboard
// shrinks the visual viewport but not the layout viewport, so the bar has to
// lift by the difference or it sits behind the keys.
//
// The limit of this gate: it drives a stand-in visual viewport, not a real
// phone. It proves the arithmetic and the wiring — what is measured, when it
// is re-read, and what is left behind on teardown.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { keyboardInset, trackKeyboard } from '../src/lib/keyboard.mjs';

/** A visual viewport that can be resized, with listeners that can be fired. */
function stubViewport(height, offsetTop = 0) {
  const listeners = new Map();
  return {
    height,
    offsetTop,
    addEventListener: (type, fn) => listeners.set(type, fn),
    removeEventListener: (type) => listeners.delete(type),
    fire: (type) => listeners.get(type)?.(),
    get tracked() {
      return [...listeners.keys()].sort();
    },
  };
}
const stubRoot = () => {
  const props = new Map();
  return { style: { setProperty: (k, v) => props.set(k, v), removeProperty: (k) => props.delete(k) }, props };
};

test('the keyboard height is what the window has that the viewport does not', () => {
  assert.equal(keyboardInset({ height: 470, offsetTop: 0 }, 844), 374);
  assert.equal(keyboardInset({ height: 844, offsetTop: 0 }, 844), 0);
});

test('a pinch-zoom pan counts as covered too, so the bar stays in view', () => {
  assert.equal(keyboardInset({ height: 400, offsetTop: 100 }, 844), 344);
});

test('a rounding pixel is not a keyboard', () => {
  assert.equal(keyboardInset({ height: 842.5, offsetTop: 0 }, 844), 0);
});

test('a browser without a visual viewport lifts the bar by nothing', () => {
  assert.equal(keyboardInset(undefined, 844), 0);
  const root = stubRoot();
  assert.equal(typeof trackKeyboard(root, { innerHeight: 844 }), 'function');
  assert.equal(root.props.size, 0);
});

test('the bar follows the keyboard opening and closing', () => {
  const viewport = stubViewport(844);
  const root = stubRoot();
  let changes = 0;
  const stop = trackKeyboard(root, { innerHeight: 844, visualViewport: viewport }, () => (changes += 1));
  assert.equal(root.props.get('--keyboard'), '0px');
  assert.deepEqual(viewport.tracked, ['resize', 'scroll']);

  viewport.height = 470;
  viewport.fire('resize');
  assert.equal(root.props.get('--keyboard'), '374px');

  viewport.height = 844;
  viewport.fire('resize');
  assert.equal(root.props.get('--keyboard'), '0px');

  // The page is told each time, so it can re-check what is still on screen.
  assert.equal(changes, 3);

  stop();
  assert.deepEqual(viewport.tracked, []);
  assert.equal(root.props.has('--keyboard'), false);
});

test('the bar is positioned by the property it sets', () => {
  const css = readFileSync(join(new URL('..', import.meta.url).pathname, 'src/styles/global.css'), 'utf8');
  const rule = /\.answer-bar\s*\{[^}]*\}/.exec(css.slice(css.indexOf('@media (max-width: 55.99rem)')))?.[0] ?? '';
  assert.match(rule, /inset-block-end:\s*calc\([^)]*var\(--keyboard, 0px\)\)/);
  assert.match(rule, /position:\s*fixed/);
});
