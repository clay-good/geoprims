// Tool pages explain their abbreviations: "Terms on this page" lists exactly
// the glossary entries for the tool (data/glossary.json), and pages with none
// have no empty section.
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const web = new URL('..', import.meta.url).pathname;
const dist = join(web, 'dist');
const catalog = JSON.parse(readFileSync(join(web, '../../dist/catalog/v1.json'), 'utf8'));
const glossary = JSON.parse(readFileSync(join(web, '../../data/glossary.json'), 'utf8'));
const route = (id) => `/${id.replaceAll('.', '/')}/`;

test('every tool page defines exactly the terms it uses', () => {
  const problems = [];
  for (const t of catalog.tools) {
    const file = join(dist, route(t.id), 'index.html');
    if (!existsSync(file)) continue;
    const html = readFileSync(file, 'utf8');
    const want = glossary.entries.filter((e) => e.relatedTools.includes(t.id)).map((e) => e.id).sort();
    const got = [...html.matchAll(/<dt id="term-([^"]+)"/g)].map((m) => m[1]).sort();
    if (JSON.stringify(got) !== JSON.stringify(want)) problems.push(`${t.id}: ${got} vs ${want}`);
    if (!want.length && html.includes('Terms on this page')) problems.push(`${t.id}: empty terms section`);
  }
  assert.deepEqual(problems, []);
});

test('the METAR decoder page defines METAR in plain words', () => {
  const html = readFileSync(join(dist, route('aviation.weather.metar-decode'), 'index.html'), 'utf8');
  assert.match(html, /<abbr title="aviation routine weather report">METAR<\/abbr>/);
  assert.match(html, /An airport weather observation in a standard code/);
});
