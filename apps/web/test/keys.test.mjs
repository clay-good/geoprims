// Global shortcuts (web/command-palette): what each key does, that single-key
// shortcuts leave text fields alone and can be turned off, and that Ctrl/Cmd+K
// always opens the palette.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { SHORTCUTS, shortcutFor } from '../src/lib/keys.js';

const field = { closest: (sel) => (sel.includes('input') ? {} : null) };
const body = { closest: () => null };
const key = (k, extra = {}) => ({ key: k, target: body, ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, ...extra });

test('slash, question mark, and g h act outside text fields', () => {
  assert.equal(shortcutFor(key('/')), 'palette');
  assert.equal(shortcutFor(key('?')), 'sheet');
  assert.equal(shortcutFor(key('g')), 'pending');
  assert.equal(shortcutFor(key('h'), { pending: true }), 'home');
  assert.equal(shortcutFor(key('h')), null, 'h alone does nothing');
});

test('in a text field, / and ? type characters; Ctrl/Cmd+K still opens the palette', () => {
  assert.equal(shortcutFor(key('/', { target: field })), null);
  assert.equal(shortcutFor(key('?', { target: field })), null);
  assert.equal(shortcutFor(key('k', { target: field, ctrlKey: true })), 'palette');
  assert.equal(shortcutFor(key('K', { target: field, metaKey: true })), 'palette');
  assert.equal(shortcutFor(key('k', { ctrlKey: true, shiftKey: true })), null, 'Ctrl+Shift+K is the browser\'s');
});

test('single-key shortcuts can be turned off (WCAG 2.1.4)', () => {
  const off = { singleKeys: false };
  assert.equal(shortcutFor(key('/'), off), null);
  assert.equal(shortcutFor(key('?'), off), null);
  assert.equal(shortcutFor(key('g'), off), null);
  assert.equal(shortcutFor(key('k', { ctrlKey: true }), off), 'palette');
});

test('the shortcut sheet lists every shortcut, and the palette offers actions', () => {
  const keys = SHORTCUTS.map(([k]) => k).join(' ');
  for (const k of ['/', 'Ctrl+K', '?', 'g then h', 'Esc', 'Enter', '>']) assert.ok(keys.includes(k), k);
  const src = readFileSync(join(new URL('..', import.meta.url).pathname, 'src/lib/palette.js'), 'utf8');
  for (const a of ['Display: ', 'Turn single-key shortcuts', 'Show keyboard shortcuts']) assert.ok(src.includes(a), a);
});

test('page shortcuts: c, y, l, s, p, [, and ] map to their actions, and each can say why it did nothing', async () => {
  const { shortcutFor, SHORTCUTS, UNAVAILABLE } = await import('../src/lib/keys.js');
  const key = (k) => shortcutFor({ key: k, target: null });
  assert.deepEqual(['c', 'y', 'l', 's', 'p', '[', ']'].map(key), ['canvas', 'copy-json', 'copy-link', 'swap', 'play', 'previous', 'next']);
  for (const a of ['canvas', 'copy-json', 'copy-link', 'swap', 'play']) assert.ok(UNAVAILABLE[a], `${a} has a message`);
  // Off when single keys are off, and never while typing.
  assert.equal(shortcutFor({ key: 'c', target: null }, { singleKeys: false }), null);
  assert.equal(shortcutFor({ key: 's', target: { closest: () => true } }), null);
  assert.equal(shortcutFor({ key: 'y', ctrlKey: true, target: null }), null, 'Ctrl+Y stays the browser’s');
  const listed = SHORTCUTS.map(([k]) => k).join(' ');
  for (const k of ['c', 'y', 'l', 's', 'p', '[ and ]']) assert.ok(listed.includes(k), `${k} is on the sheet`);
});
