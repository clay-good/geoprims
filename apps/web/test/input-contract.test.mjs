// The numeric input contract (ux/mobile-and-field). A phone keypad and a
// desktop spinner disagree about what a number is: type="number" rejects the
// units our inputs carry ("29.92 inHg") and silently scrolls to a new value
// under a stray wheel event. So every field is text, and the keypad is chosen
// with inputmode instead. This gate scans the rendered forms.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { isNumeric, isSigned, flipped } from '../src/lib/fields.mjs';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const dist = join(web, 'dist');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const route = (id) => '/' + id.split('.').join('/') + '/';
const page = (r) => readFileSync(join(dist, r, 'index.html'), 'utf8');

/** The rendered `<input id="field-NAME">`, attributes and all. */
const control = (html, name) => new RegExp(`<input[^>]*\\bid="field-${name}"[^>]*>`).exec(html)?.[0] ?? '';
const attr = (tag, name) => new RegExp(`\\b${name}="([^"]*)"`).exec(tag)?.[1];

test('no field is a number input', () => {
  const problems = [];
  for (const t of catalog.tools) {
    for (const tag of page(route(t.id)).match(/<(input|textarea)\b[^>]*>/g) ?? []) {
      if (attr(tag, 'type') === 'number') problems.push(`${t.id}: ${attr(tag, 'id')} is type="number"`);
    }
  }
  assert.deepEqual(problems, []);
});

test('every numeric field asks for the decimal keypad', () => {
  const problems = [];
  let checked = 0;
  for (const t of catalog.tools) {
    const html = page(route(t.id));
    for (const [name, schema] of Object.entries(t.inputs.properties)) {
      if (name === 'options' || schema.enum || schema.type === 'array' || !isNumeric(schema)) continue;
      const tag = control(html, name);
      if (!tag) continue;
      checked += 1;
      if (attr(tag, 'inputmode') !== 'decimal') problems.push(`${t.id}: ${name} has no inputmode="decimal"`);
    }
  }
  assert.deepEqual(problems, []);
  assert.ok(checked > 200, `only ${checked} numeric fields scanned`);
});

test('no field fights what the user typed', () => {
  const problems = [];
  for (const t of catalog.tools) {
    for (const tag of page(route(t.id)).match(/<(input|textarea)\b[^>]*\bid="field-[^"]*"[^>]*>/g) ?? []) {
      for (const off of ['autocomplete', 'autocorrect', 'spellcheck']) {
        if (attr(tag, off) !== (off === 'spellcheck' ? 'false' : 'off')) {
          problems.push(`${t.id}: ${attr(tag, 'id')} does not turn ${off} off`);
        }
      }
    }
  }
  assert.deepEqual(problems, []);
});

test('the keyboard says whether another field follows', () => {
  const problems = [];
  for (const t of catalog.tools) {
    const html = page(route(t.id));
    const names = Object.keys(t.inputs.properties).filter((n) => n !== 'options');
    const last = names.at(-1);
    for (const name of names) {
      const tag = control(html, name);
      if (!tag) continue;
      const want = name === last ? 'done' : 'next';
      const got = attr(tag, 'enterkeyhint');
      if (got !== want) problems.push(`${t.id}: ${name} has enterkeyhint="${got}", expected "${want}"`);
    }
  }
  assert.deepEqual(problems, []);
});

test('a value that can go negative gets a sign toggle', () => {
  const problems = [];
  let toggles = 0;
  for (const t of catalog.tools) {
    const html = page(route(t.id));
    for (const [name, schema] of Object.entries(t.inputs.properties)) {
      if (name === 'options' || schema.enum || schema.type === 'array' || !isSigned(name, schema)) continue;
      if (!control(html, name)) continue;
      // The toggle sits beside its input, inside the same .entry wrapper.
      // Svelte writes hydration comments between the two, so skip anything
      // that is not another element.
      const entry = new RegExp(`<input[^>]*\\bid="field-${name}"[^>]*>(?:\\s|<!--[^>]*-->)*<button[^>]*class="sign"`).test(html);
      if (entry) toggles += 1;
      else problems.push(`${t.id}: ${name} can be negative but has no sign toggle`);
    }
  }
  assert.deepEqual(problems, []);
  assert.ok(toggles > 30, `only ${toggles} sign toggles rendered`);
});

test('text stays at least 16 px, so no phone zooms the form', () => {
  const css = readFileSync(join(web, 'src/styles/global.css'), 'utf8');
  const rule = /input,\s*textarea,\s*select\s*{[^}]*}|input,\s*textarea\s*{[^}]*}/.exec(css)?.[0] ?? '';
  const size = /font-size:\s*([\d.]+)(rem|px)/.exec(rule);
  assert.ok(size, 'no font-size on the shared input rule');
  const px = size[2] === 'rem' ? Number(size[1]) * 16 : Number(size[1]);
  assert.ok(px >= 16, `input font-size is ${px}px, below the 16px zoom threshold`);
});

test('pinch zoom is never taken away', () => {
  const problems = [];
  for (const t of catalog.tools.slice(0, 40)) {
    const viewport = /<meta[^>]*name="viewport"[^>]*>/.exec(page(route(t.id)))?.[0] ?? '';
    if (/user-scalable\s*=\s*no|maximum-scale\s*=\s*1/.test(viewport)) problems.push(`${t.id}: ${viewport}`);
  }
  assert.deepEqual(problems, []);
});

test('the sign toggle keeps the unit', () => {
  assert.equal(flipped('30 °C'), '-30 °C');
  assert.equal(flipped('-30 °C'), '30 °C');
  assert.equal(flipped(''), '-');
});

test('the gate bites', () => {
  const bad = '<input id="field-elevation" type="number" inputmode="numeric" spellcheck="true">';
  assert.equal(attr(bad, 'type'), 'number');
  assert.notEqual(attr(bad, 'inputmode'), 'decimal');
  assert.notEqual(attr(bad, 'spellcheck'), 'false');
  assert.equal(attr(bad, 'enterkeyhint'), undefined);
});
