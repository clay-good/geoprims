// Touch targets (ux/mobile-and-field). Every control is at least 48 × 48 CSS
// px with 8 px between neighbours, and Field mode raises that to 56.
//
// The limit of this gate: it reads the declared rules, not a laid-out page —
// there is no browser in this build. It holds because one rule sizes every
// control from the --target token; so it checks that the token is right, that
// the rule covers every kind of control the pages actually render, and that
// nothing else sizes a control below it.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
// Comments come out first, or one would ride along in the next selector.
const css = readFileSync(join(web, 'src/styles/global.css'), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const page = (id) => readFileSync(join(web, 'dist', ...id.split('.'), 'index.html'), 'utf8');

const MIN = 48;
const FIELD_MIN = 56;
const SPACING = 8;

/** Every `selector { declarations }` pair, in source order. */
const rules = [...css.matchAll(/([^{}]+)\{([^{}]*)\}/g)].map(([, sel, body]) => ({
  selector: sel.trim().replace(/\s+/g, ' '),
  body: body.trim(),
}));
const declared = (body, prop) => new RegExp(`(?:^|;)\\s*${prop}\\s*:\\s*([^;]+)`).exec(body)?.[1].trim();
/** A CSS length in px, or null when it is not a plain length. */
const px = (value) => {
  const m = /^([\d.]+)(px|rem)$/.exec(value ?? '');
  return m ? Number(m[1]) * (m[2] === 'rem' ? 16 : 1) : null;
};
const token = (selector, name) => declared(rules.find((r) => r.selector === selector)?.body ?? '', name);

test('the target token is 48 px, and 56 px in Field mode', () => {
  assert.equal(px(token(':root', '--target')), MIN);
  assert.equal(px(token(":root[data-field='on']", '--target')), FIELD_MIN);
});

test('one rule sizes every control from the token', () => {
  const rule = rules.find((r) => r.selector === 'input, select, textarea, button');
  assert.ok(rule, 'no shared rule for input, select, textarea and button');
  assert.equal(declared(rule.body, 'min-block-size'), 'var(--target)');
});

test('nothing sizes a control below the target', () => {
  const problems = [];
  for (const { selector, body } of rules) {
    if (!/(^|[\s,>])(input|select|textarea|button|summary)\b/.test(selector)) continue;
    for (const prop of ['min-block-size', 'block-size', 'height', 'min-height']) {
      const value = declared(body, prop);
      const size = px(value);
      if (size === null || size >= MIN) continue;
      // A zero minimum is fine when the same rule sets the size outright.
      if (size === 0 && /var\(--target\)/.test(declared(body, 'block-size') ?? '')) continue;
      problems.push(`${selector}: ${prop}: ${value}`);
    }
  }
  assert.deepEqual(problems, []);
});

test('controls that sit side by side keep 8 px between them', () => {
  const problems = [];
  for (const selector of ['.actions', '.steps', '.entry']) {
    const body = rules.find((r) => r.selector === selector)?.body;
    assert.ok(body, `no rule for ${selector}`);
    const gap = declared(body, 'gap')?.split(/\s+/).map(px) ?? [];
    for (const size of gap) {
      if (size === null || size < SPACING) problems.push(`${selector}: gap ${declared(body, 'gap')}`);
    }
  }
  assert.deepEqual(problems, []);
});

test('the pages render no control the rule does not cover', () => {
  const covered = new Set(['input', 'select', 'textarea', 'button', 'summary', 'a', 'dialog', 'details', 'form', 'label']);
  const problems = new Set();
  for (const t of catalog.tools.slice(0, 60)) {
    for (const [, tag] of page(t.id).matchAll(/<([a-z]+)[^>]*\b(?:onclick|tabindex|role="button")/g)) {
      if (!covered.has(tag)) problems.add(`${t.id}: <${tag}> is interactive but not a control`);
    }
  }
  assert.deepEqual([...problems], []);
});

test('Field mode raises the answer text as well as the targets', () => {
  const rule = rules.find((r) => r.selector.includes("[data-field='on'] .answer .value"));
  assert.ok(rule, 'Field mode does not raise the answer text');
  assert.equal(declared(rule.body, 'font-size'), '1.25em');
});

test('the step buttons appear only in Field mode', () => {
  assert.equal(declared(rules.find((r) => r.selector === '.steps').body, 'display'), 'none');
  assert.equal(declared(rules.find((r) => r.selector === ":root[data-field='on'] .steps").body, 'display'), 'flex');
});

test('the gate bites', () => {
  const tiny = [{ selector: 'button.tiny', body: 'min-block-size: 32px' }];
  const under = tiny.filter((r) => px(declared(r.body, 'min-block-size')) < MIN);
  assert.equal(under.length, 1);
  assert.equal(px('3rem'), 48);
  assert.equal(px('auto'), null);
});
