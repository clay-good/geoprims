// Layouts for every width (ux/mobile-and-field). The narrowest phone and the
// narrowest split screen are both 320 px, and nothing may push a page wider
// than the screen it is on.
//
// The limit of this gate: it reads the declared rules and the built pages, not
// a laid-out viewport — there is no browser in this build. It catches what
// actually causes horizontal scroll in this stylesheet: a length wider than
// the screen, a column that cannot get narrower, and a wide element with no
// way to scroll inside its own box.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const css = readFileSync(join(web, 'src/styles/global.css'), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const page = (id) => readFileSync(join(web, 'dist', ...id.split('.'), 'index.html'), 'utf8');

/** The narrowest screen a page has to fit, and what is left after the gutter. */
const NARROWEST = 320;
const GUTTER = 12 * 2;
const CONTENT = NARROWEST - GUTTER;

const rules = [...css.matchAll(/([^{}@]+)\{([^{}]*)\}/g)].map(([, sel, body]) => ({
  selector: sel.trim().replace(/\s+/g, ' '),
  body: body.trim(),
}));
const px = (value) => {
  const m = /^([\d.]+)(px|rem|em)$/.exec(String(value).trim());
  return m ? Number(m[1]) * (m[2] === 'px' ? 1 : 16) : null;
};

// Something genuinely wider than a phone — a verification table of digests —
// is allowed, but only inside a box that scrolls on its own.
const WIDE_BY_DESIGN = new Set(['table.report']);

test('no rule sets a width the narrowest screen cannot hold', () => {
  const problems = [];
  for (const { selector, body } of rules) {
    if (WIDE_BY_DESIGN.has(selector)) continue;
    for (const [, prop, value] of body.matchAll(/(?:^|;)\s*(min-inline-size|inline-size|width|min-width)\s*:\s*([^;]+)/g)) {
      const size = px(value);
      if (size !== null && size > CONTENT) problems.push(`${selector}: ${prop}: ${value.trim()}`);
    }
  }
  assert.deepEqual(problems, []);
});

test('everything wide by design sits in a box that scrolls', () => {
  const problems = [];
  let found = 0;
  for (const path of ['agents/index.html', 'verification/0.1.0/index.html']) {
    const html = readFileSync(join(web, 'dist', path), 'utf8');
    for (const [i] of [...html.matchAll(/<table class="report"/g)].map((m) => [m.index])) {
      found += 1;
      const before = html.slice(0, i);
      const wrapper = before.lastIndexOf('<div class="table-scroll"');
      if (wrapper === -1 || before.indexOf('</div>', wrapper) !== -1) {
        problems.push(`${path}: a report table outside a scrolling box`);
      }
    }
  }
  assert.deepEqual(problems, []);
  assert.ok(found >= 4, `only ${found} report tables checked`);
});

test('no grid column refuses to get narrower than the screen', () => {
  const problems = [];
  for (const { selector, body } of rules) {
    for (const [, min] of body.matchAll(/minmax\(\s*([^,)]+),/g)) {
      const size = px(min);
      if (size !== null && size > CONTENT) problems.push(`${selector}: minmax(${min.trim()}, …)`);
    }
  }
  assert.deepEqual(problems, []);
});

test('a page that is too wide for the screen scrolls inside its own box', () => {
  const scrolls = (selector) => {
    const body = rules.filter((r) => r.selector === selector).map((r) => r.body).join(';');
    return /overflow(-x|-inline)?\s*:\s*(auto|scroll)/.test(body);
  };
  for (const selector of ['.table-scroll', 'pre.snippet']) {
    assert.ok(scrolls(selector), `${selector} has no way to scroll a wide child`);
  }
});

test('the wide layout starts at a tablet, not before', () => {
  // Inputs and answer sit side by side only where both fit.
  const wide = /@media \(min-width: ([\d.]+)rem\)\s*\{[^@]*\.tool-grid/.exec(css);
  assert.ok(wide, 'no side-by-side rule for the tool grid');
  const at = Number(wide[1]) * 16;
  assert.ok(at >= 880 && at <= 920, `side by side starts at ${at}px, which is not about 900`);
});

test('the phone layout covers every split-screen width', () => {
  // The sticky bar and the single column belong to everything below the wide
  // breakpoint, so a 320-500 px split screen gets the phone layout.
  const narrow = /@media \(max-width: ([\d.]+)rem\)\s*\{[^@]*\.answer-bar/.exec(css);
  assert.ok(narrow, 'no phone rule for the answer bar');
  assert.ok(Number(narrow[1]) * 16 >= 500, 'the phone layout stops before a split screen does');
});

test('no page carries a fixed width of its own', () => {
  const problems = [];
  for (const t of catalog.tools.slice(0, 60)) {
    for (const [, style] of page(t.id).matchAll(/style="([^"]*)"/g)) {
      for (const [, value] of style.matchAll(/(?:^|;)\s*(?:min-)?width\s*:\s*([^;]+)/g)) {
        const size = px(value);
        if (size !== null && size > CONTENT) problems.push(`${t.id}: width: ${value}`);
      }
    }
  }
  assert.deepEqual(problems, []);
});

test('the gate bites', () => {
  assert.equal(px('20rem'), 320);
  assert.ok(px('20rem') > CONTENT, 'a 320 px column would overflow a 320 px screen');
  assert.equal(px('auto'), null);
});
