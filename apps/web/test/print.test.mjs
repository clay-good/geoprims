// The one-page calculation sheet (ux/mobile-and-field, "Printing and sharing
// from mobile"). Printing a tool page should produce a sheet, not a screenshot
// of an app: the tool, the inputs as typed, the answer with its sentence, and
// where the method comes from — and none of the controls.
//
// This gate reads the print rules and the built pages. The rendered sheet is
// checked in a real browser by test/browser/print.test.mjs, which prints a PDF
// and counts its pages.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { sharePayload, copyText } from '../src/lib/copy.js';

const web = new URL('..', import.meta.url).pathname;
const css = readFileSync(join(web, 'src/styles/global.css'), 'utf8');
/** The declarations inside the print block that hides the app's controls. */
const sheet = /@media print \{\s*header\.site,([\s\S]*?)\n\}/.exec(css)?.[1] ?? '';

/** The selectors the sheet hides outright, as written. */
const hides = new Set(
  (/([^{}]*)\{[^}]*display: none !important/.exec(sheet)?.[1] ?? '')
    .split(',')
    .map((s) => s.trim().replace(/\s+/g, ' '))
    .filter(Boolean),
);
const hidden = (selector) => hides.has(selector);

test('the sheet carries no control and no navigation', () => {
  for (const selector of ['.actions', '.crumbs', 'footer.site nav', '.answer-bar', '.steps', '.update', 'details.dev']) {
    assert.ok(hidden(selector), `${selector} is still printed`);
  }
  assert.match(css, /@media print \{\s*header\.site/, 'the site header is still printed');
});

test('the sheet keeps the answer, the inputs, and the terms', () => {
  for (const selector of ['.card.answer', '.card.inputs', 'details.card.terms', '.sentence', '.facts']) {
    assert.ok(!hidden(selector), `${selector} is hidden from the sheet`);
  }
  // Inputs print as the values they hold, without the box around them.
  assert.match(sheet, /input, select, textarea \{[^}]*border: 0/);
});

test('a sheet printed from an example still says it is an example', () => {
  // The example chip is the only thing standing between a printed sheet and a
  // reader taking prefilled values for their own.
  assert.ok(!hidden('.chip'), 'the example chip is hidden from the sheet');
  assert.ok(hidden('.chip.offline'), 'the offline chip is printed');
});

test('the sheet fits a page', () => {
  assert.match(sheet, /@page \{ margin: [\d.]+mm; \}/);
  // Nothing may be split across two pages mid-card.
  assert.match(sheet, /\.card \{[^}]*break-inside: avoid/);
});

test('printing uses the paper palette whichever mode the screen is in', () => {
  const print = /@media print \{\s*:root,\s*:root\[data-theme\] \{([^}]*)\}/.exec(css)?.[1] ?? '';
  assert.match(print, /--bg: #ffffff/);
  assert.match(print, /--text: #15171c/);
});

test('the share sheet offers the sentence, its reference, and the link', () => {
  const parts = {
    answer: '7,120 ft',
    result: { summary: 'Density altitude is 7,120 ft.' },
    tool: { id: 'aviation.altimetry.density-altitude', title: 'Density altitude', version: '1.2.0' },
    args: {},
    href: 'https://geoprims.com/aviation/altimetry/density-altitude/#v1:abc',
  };
  const payload = sharePayload(parts);
  assert.equal(payload.title, 'Density altitude · geoprims');
  assert.equal(payload.url, parts.href);
  assert.match(payload.text, /Density altitude is 7,120 ft\./);
  assert.match(payload.text, /aviation\.altimetry\.density-altitude 1\.2\.0/);
  // A share and a copy say the same thing.
  assert.equal(payload.text, copyText('sentence', parts));
});

test('a browser with no share sheet still copies the link', () => {
  const app = readFileSync(join(web, 'src/components/ToolApp.svelte'), 'utf8');
  assert.match(app, /if \(navigator\.share\)[\s\S]*?await copy\('link'\)/);
});
