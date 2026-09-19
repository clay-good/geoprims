// Claims honesty and review disclosure (trust/correctness-program 3.6, 3.7),
// run over the built site and the READMEs.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { claimProblems } from '../../../tools/trust/claims.mjs';
import { checkSignoffs, parseSignoffs, readSignoffs, reviewSentence } from '../../../tools/trust/signoffs.mjs';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const dist = join(web, 'dist');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const signoffs = readSignoffs(root);
const ctx = { catalog, signoffs, root, offline: existsSync(join(dist, 'sw.js')) };
const today = new Date().toISOString().slice(0, 10);

function htmlFiles(dir, out = []) {
  for (const f of readdirSync(dir)) {
    const p = join(dir, f);
    if (statSync(p).isDirectory()) htmlFiles(p, out);
    else if (f.endsWith('.html')) out.push(p);
  }
  return out;
}

/** The README's statements about what is built (the principles describe the specs). */
const readmeProgress = () => {
  const md = readFileSync(join(root, 'README.md'), 'utf8');
  return md.slice(md.indexOf('## Progress'), md.indexOf('## Building it'));
};

test('public claims match the build', () => {
  const problems = [];
  for (const f of htmlFiles(dist)) problems.push(...claimProblems(readFileSync(f, 'utf8'), f.slice(dist.length), ctx));
  for (const f of ['llms.txt', 'AGENTS.md']) problems.push(...claimProblems(readFileSync(join(dist, f), 'utf8'), f, ctx));
  problems.push(...claimProblems(readmeProgress(), 'README.md', ctx));
  for (const f of ['mcp/README.md', 'apps/web/README.md', 'worker/README.md']) {
    problems.push(...claimProblems(readFileSync(join(root, f), 'utf8'), f, ctx));
  }
  assert.deepEqual(problems, []);
});

test('an overclaim is caught and quoted', () => {
  const text = 'Every tool is checked against GeographicLib. All tools are marked experimental. It works offline. Reviewed by licensed surveyors.';
  const p = claimProblems(text, 'fixture', { ...ctx, offline: false });
  assert.equal(p.length, 4, p.join('\n'));
  assert.match(p[0], /claims every tool is checked against a reference.*"Every tool is checked against GeographicLib\."/);
  assert.match(p[1], /claims everything is experimental/);
  assert.match(p[2], /no service worker/);
  assert.match(p[3], /does not record/);
  const counts = claimProblems('geoprims has 3 operations and 4 tool ids.', 'fixture', ctx);
  assert.match(counts[0], new RegExp(`the build has ${catalog.counts.all.operations}`));
});

test('sign-off records are valid, and unreviewed domains say so on their pages', () => {
  assert.deepEqual(checkSignoffs(signoffs, catalog, today), []);
  for (const d of new Set(catalog.tools.map((t) => t.domain))) {
    const line = reviewSentence(signoffs, d, today);
    const html = readFileSync(join(dist, d, 'index.html'), 'utf8');
    assert.ok(html.includes(line.replaceAll("'", '&#39;')), `${d}: missing "${line}"`);
  }
  const survey = reviewSentence(signoffs, 'survey', today);
  if (!signoffs.records.some((r) => r.domain === 'survey')) assert.equal(survey, 'Not yet independently reviewed by a licensed surveyor.');
  const tool = catalog.tools.find((t) => t.domain === 'survey');
  const page = readFileSync(join(dist, ...tool.id.split('.'), 'index.html'), 'utf8');
  assert.ok(page.includes(reviewSentence(signoffs, 'survey', today, tool.id)), 'the tool proof panel states the review status');
});

test('a sign-off is honored, then expires after 12 months', () => {
  const md = readFileSync(join(root, 'docs/review-signoffs.md'), 'utf8') +
    '| survey | Pat Example | Licensed surveyor | all | 2026-01-15 | Traverse and area tools |\n';
  const s = parseSignoffs(md);
  assert.equal(reviewSentence(s, 'survey', '2026-09-19'), 'Reviewed by Pat Example (Licensed surveyor) on 2026-01-15: Traverse and area tools.');
  assert.deepEqual(claimProblems('Reviewed by Pat Example.', 'fixture', { ...ctx, signoffs: s }), []);
  assert.match(reviewSentence(s, 'survey', '2027-02-01'), /^Not yet independently reviewed/);
  assert.deepEqual(checkSignoffs(s, catalog, '2027-02-01'), ['sign-off by Pat Example for survey expired (over 12 months)']);
});
