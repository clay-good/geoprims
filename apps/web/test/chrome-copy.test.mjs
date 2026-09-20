// Plain wording for chrome (contracts/page-chrome). Buttons, headings, and
// summaries are written in sentence case and never shout: no chrome element is
// all capitals for emphasis. Acronyms and unit symbols are exempt, and the
// glossary is the list of acronyms the site is allowed to use.
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const dist = join(web, 'dist');
const glossary = JSON.parse(readFileSync(join(root, 'data/glossary.json'), 'utf8'));

/** Acronyms the site may print in capitals: every glossary term, plus these. */
export const ALLOWED = new Set([
  ...glossary.entries.map((e) => e.term.toUpperCase()),
  ...(glossary.notAbbreviations ?? []).map((n) => n.token),
  'US', 'UK', 'GPS', 'JSON', 'CSV', 'URL', 'API', 'MCP', 'PWA', 'ID', 'OK', 'AM', 'PM', 'UTC', 'AGL', 'MSL',
]);

/** Chrome elements: the controls and headings that frame a page. */
const CHROME = /<(button|summary|h1|h2|h3|legend|th)\b[^>]*>([\s\S]*?)<\/\1>/g;

const textOf = (html) =>
  html
    .replace(/<!--[\s\S]*?-->/g, '')
    .replace(/<[^>]+>/g, ' ')
    .replace(/&[a-z]+;|&#\d+;/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();

/**
 * The capitalised words of a string that shouts. A string shouts when every
 * word in it is capitals and at least one is a real word rather than an
 * acronym. Strings carrying digits are values a tool produced (an MGRS
 * reference, a GPX route), not chrome copy, so they are left alone.
 */
export function shouting(text) {
  if (/\d/.test(text)) return [];
  const words = text.match(/\b[A-Za-z][A-Za-z'’]*\b/g) ?? [];
  if (words.length === 0) return [];
  if (words.some((w) => w !== w.toUpperCase())) return [];
  const loud = words.filter((w) => w.length > 1 && !ALLOWED.has(w));
  return loud.some((w) => w.length >= 4) ? loud : [];
}

function htmlFiles(dir, out = []) {
  for (const f of readdirSync(dir)) {
    const p = join(dir, f);
    if (statSync(p).isDirectory()) htmlFiles(p, out);
    else if (f.endsWith('.html')) out.push(p);
  }
  return out;
}

test('no chrome element shouts', () => {
  const problems = [];
  for (const file of htmlFiles(dist)) {
    const html = readFileSync(file, 'utf8');
    for (const [, , inner] of html.matchAll(CHROME)) {
      const text = textOf(inner);
      if (!text) continue;
      const loud = shouting(text);
      if (loud.length) problems.push(`${file.slice(dist.length)}: "${text}" shouts ${loud.join(', ')}`);
    }
  }
  assert.deepEqual([...new Set(problems)], []);
});

test('the lint catches an all-caps control and leaves acronyms alone', () => {
  assert.deepEqual(shouting('REPORT A PROBLEM'), ['REPORT', 'PROBLEM']);
  assert.deepEqual(shouting('WARNING'), ['WARNING']);
  assert.deepEqual(shouting('Report a problem'), []);
  assert.deepEqual(shouting('Copy the MGRS reference'), [], 'a glossary acronym is fine');
  assert.deepEqual(shouting('IFR'), [], 'an acronym on its own is not shouting');
  assert.deepEqual(shouting('VS Code'), [], 'a product name is not shouting');
  assert.deepEqual(shouting('17T NE 86309 77770'), [], 'a result value is not chrome copy');
  assert.deepEqual(shouting('Density altitude in ft'), []);
});
