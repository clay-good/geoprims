// Keeping the sticky answer above the on-screen keyboard
// (ux/mobile-and-field, "Sticky answer above the keyboard").
//
// A phone keyboard does not shrink the layout viewport, so an element pinned
// to the bottom of the page sits behind it. The visual viewport does shrink,
// and the gap between the two is the height the keyboard covers. That gap
// goes on the document as --keyboard, and the bar lifts by it.

/** How much of the window the keyboard (or a pinch-zoom pan) covers, in px. */
export function keyboardInset(viewport, windowHeight) {
  if (!viewport || !Number.isFinite(windowHeight)) return 0;
  const covered = windowHeight - viewport.height - (viewport.offsetTop ?? 0);
  // A pixel or two of rounding is not a keyboard.
  return covered > 2 ? Math.round(covered) : 0;
}

/**
 * Keeps `--keyboard` on `root` in step with the visual viewport, calling
 * `onChange` whenever it moves. Returns a function that stops tracking and
 * clears the property.
 */
export function trackKeyboard(root, win = globalThis, onChange) {
  const viewport = win.visualViewport;
  if (!root || !viewport) return () => {};
  const update = () => {
    root.style.setProperty('--keyboard', `${keyboardInset(viewport, win.innerHeight)}px`);
    onChange?.();
  };
  update();
  viewport.addEventListener('resize', update);
  viewport.addEventListener('scroll', update);
  return () => {
    viewport.removeEventListener('resize', update);
    viewport.removeEventListener('scroll', update);
    root.style.removeProperty('--keyboard');
  };
}
