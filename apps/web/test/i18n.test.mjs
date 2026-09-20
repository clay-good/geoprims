// Logical properties and the message catalog (web/visual-theme,
// "Externalized strings and logical properties"). The site is US English
// today, and stays translatable by not baking the writing direction into the
// layout and by keeping a shared string in one place.
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import * as messages from '../src/lib/messages.js';

const web = new URL('..', import.meta.url).pathname;
const src = join(web, 'src');
const dist = join(web, 'dist');

/** A physical direction in a property a logical one covers. */
export const PHYSICAL = [
  /\b(?:margin|padding|border|inset|scroll-margin|scroll-padding)-(?:left|right)\b/,
  /\btext-align:\s*(?:left|right)\b/,
  /\bborder-(?:top|bottom)-(?:left|right)-radius\b/,
  /(?<![-\w])(?:left|right):\s*(?!auto\b)/,
];

function files(dir, ext, out = []) {
  for (const f of readdirSync(dir)) {
    const p = join(dir, f);
    if (statSync(p).isDirectory()) files(p, ext, out);
    else if (ext.some((e) => f.endsWith(e))) out.push(p);
  }
  return out;
}

/** Every physical-direction declaration in a stylesheet, with its line. */
export function physical(css) {
  const out = [];
  css.split('\n').forEach((line, i) => {
    // A comment explaining why is allowed to name one.
    if (line.trim().startsWith('/*') || line.trim().startsWith('*')) return;
    for (const rule of PHYSICAL) {
      const m = rule.exec(line);
      if (m) out.push({ line: i + 1, text: m[0].trim() });
    }
  });
  return out;
}

test('no stylesheet bakes in a writing direction', () => {
  const problems = [];
  for (const file of files(src, ['.css', '.svelte', '.astro'])) {
    for (const p of physical(readFileSync(file, 'utf8'))) {
      problems.push(`${file.slice(src.length)}:${p.line} uses ${p.text}`);
    }
  }
  assert.deepEqual(problems, []);
});

test('the lint catches a physical property, and leaves the logical one alone', () => {
  assert.deepEqual(physical('.x { margin-left: 1rem; }').map((p) => p.text), ['margin-left']);
  assert.deepEqual(physical('.x { text-align: left; }').map((p) => p.text), ['text-align: left']);
  assert.deepEqual(physical('.x { left: 0; }').map((p) => p.text), ['left:']);
  assert.deepEqual(physical('.x { margin-inline-start: 1rem; text-align: start; inset-inline-start: 0; }'), []);
  // A word that merely contains one is not a match.
  assert.deepEqual(physical('.x { border-inline-start-width: 1px; }'), []);
});

test('the built stylesheets carry no physical direction either', () => {
  const problems = [];
  for (const file of files(join(dist, '_astro'), ['.css'])) {
    for (const p of physical(readFileSync(file, 'utf8'))) problems.push(`${file.slice(dist.length)}: ${p.text}`);
  }
  assert.deepEqual(problems, []);
});

test('a message shown in more than one place lives in the catalog', () => {
  const shared = Object.entries(messages).filter(([, v]) => typeof v === 'string');
  assert.ok(shared.length > 0, 'the catalog is empty');
  const sources = files(src, ['.js', '.svelte', '.astro']).filter((f) => !f.endsWith('messages.js'));
  for (const [name, text] of shared) {
    const inline = sources.filter((f) => readFileSync(f, 'utf8').includes(text.slice(0, 40)));
    assert.deepEqual(inline, [], `${name} is written out again in ${inline.map((f) => f.slice(src.length)).join(', ')}`);
  }
});
