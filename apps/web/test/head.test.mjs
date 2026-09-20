// Titles and descriptions from one source (discovery/search-pages). Every
// page's head goes through src/lib/head.mjs, so no template can ship a title
// or description a search engine would cut, repeat another page's title, or
// promise something the build cannot prove.
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { DESCRIPTION_MAX, TITLE_MAX, capTitle, description, headProblems, superlative, title } from '../src/lib/head.mjs';

const web = new URL('..', import.meta.url).pathname;
const dist = join(web, 'dist');

function htmlFiles(dir, out = []) {
  for (const f of readdirSync(dir)) {
    const p = join(dir, f);
    if (statSync(p).isDirectory()) htmlFiles(p, out);
    else if (f.endsWith('.html')) out.push(p);
  }
  return out;
}

const pages = htmlFiles(dist).map((f) => {
  const html = readFileSync(f, 'utf8');
  return {
    route: f.slice(dist.length),
    title: /<title>([^<]*)<\/title>/.exec(html)?.[1] ?? '',
    description: /<meta name="description" content="([^"]*)"/.exec(html)?.[1] ?? '',
  };
});

test('every built page has a title and description within the caps, unique, and plain', () => {
  assert.ok(pages.length > 100, 'found the built pages');
  assert.deepEqual(headProblems(pages), []);
});

test('the title cap drops the qualifier before shortening the name', () => {
  assert.equal(title('Density altitude', 'how high the airplane feels'), 'Density altitude — how high the airplane feels · geoprims');
  const long = title('Density altitude', 'how high the airplane feels on a hot day at a high field');
  assert.ok(long.length <= TITLE_MAX, long);
  assert.equal(long, 'Density altitude · geoprims', 'the qualifier goes, the name stays whole');
  const veryLong = title('A tool with an extremely long name that will not fit inside the cap at all');
  assert.ok(veryLong.length <= TITLE_MAX, veryLong);
  assert.ok(veryLong.endsWith('… · geoprims'));
});

test('capTitle holds a title a template composed itself', () => {
  assert.equal(capTitle('Short · geoprims'), 'Short · geoprims');
  const capped = capTitle('Move a position between epochs (ITRF2020 plate motion) · geoprims');
  assert.ok(capped.length <= TITLE_MAX, capped);
  assert.ok(capped.endsWith(' · geoprims'));
});

test('a description is cut at a sentence, then at a word', () => {
  assert.equal(description('Short enough.'), 'Short enough.');
  const twoSentences = `${'x'.repeat(100)}. ${'y'.repeat(100)}.`;
  assert.equal(description(twoSentences), `${'x'.repeat(100)}.`, 'the first sentence is enough');
  const oneLong = `${'word '.repeat(60)}end.`;
  const cut = description(oneLong);
  assert.ok(cut.length <= DESCRIPTION_MAX, `${cut.length}`);
  assert.ok(cut.endsWith('…'));
  assert.ok(!cut.endsWith(' …'), 'no space before the ellipsis');
});

test('the lint rejects a page that promises more than the build can prove', () => {
  assert.equal(superlative('The best crosswind calculator'), 'best');
  assert.equal(superlative('The most accurate geoid model'), 'most accurate');
  assert.equal(superlative('#1 for pilots'), '#1');
  assert.equal(superlative('Exact, cited crosswind components'), undefined);
  assert.equal(superlative('Bestimmung'), undefined, 'not a word that merely contains one');
  assert.deepEqual(headProblems([{ route: '/x/', title: 'The best tool · geoprims', description: 'Fine.' }]), [
    '/x/: the title says "best"',
  ]);
});

test('the lint catches two pages sharing a title, and a head over the caps', () => {
  const same = [
    { route: '/a/', title: 'Same · geoprims', description: 'One.' },
    { route: '/b/', title: 'Same · geoprims', description: 'Two.' },
  ];
  assert.deepEqual(headProblems(same), ['/b/ and /a/ share a title']);
  const long = [{ route: '/c/', title: 'x'.repeat(TITLE_MAX + 1), description: 'y'.repeat(DESCRIPTION_MAX + 1) }];
  assert.deepEqual(headProblems(long), [
    `/c/: title is ${TITLE_MAX + 1} characters (at most ${TITLE_MAX})`,
    `/c/: description is ${DESCRIPTION_MAX + 1} characters (at most ${DESCRIPTION_MAX})`,
  ]);
});
