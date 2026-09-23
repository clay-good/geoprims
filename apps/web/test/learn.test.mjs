// Concept explainers (discovery/search-pages, "Concept explainers and
// journeys"): each is a cited Article with a live example, linked from /learn/
// and from every tool it names, and long enough to teach something.
import assert from 'node:assert/strict';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';

const dist = join(new URL('..', import.meta.url).pathname, 'dist');
const content = join(new URL('..', import.meta.url).pathname, 'src/content/learn');
const slugs = readdirSync(content).filter((f) => f.endsWith('.md')).map((f) => f.slice(0, -3));
const page = (route) => readFileSync(join(dist, route, 'index.html'), 'utf8');
const words = (html) => html.replace(/<[^>]+>/g, ' ').split(/\s+/).filter((w) => /[a-z]/i.test(w)).length;

/** The minimum the launch plan asks for (plan-launch-and-value-proof L1). */
export const MIN_EXPLAINERS = 25;

test(`at least ${MIN_EXPLAINERS} explainers are published`, () => {
  assert.ok(slugs.length >= MIN_EXPLAINERS, `${slugs.length} explainers`);
});

test('every explainer is a cited, dated Article with a live example', () => {
  const problems = [];
  for (const slug of slugs) {
    const html = page(`learn/${slug}`);
    const ld = [...html.matchAll(/<script type="application\/ld\+json">([\s\S]*?)<\/script>/g)].map((m) => JSON.parse(m[1].replaceAll('\\u003c', '<')));
    const article = ld.find((o) => o['@type'] === 'Article');
    if (!article) { problems.push(`${slug}: no Article`); continue; }
    if (!/^\d{4}-\d{2}-\d{2}$/.test(article.datePublished ?? '')) problems.push(`${slug}: datePublished ${article.datePublished}`);
    if (!/^\d{4}-\d{2}-\d{2}$/.test(article.dateModified ?? '')) problems.push(`${slug}: dateModified ${article.dateModified}`);
    const body = /<article class="prose">([\s\S]*?)<\/article>/.exec(html)?.[1] ?? '';
    if (words(body) < 600) problems.push(`${slug}: ${words(body)} words, under 600`);
    const sources = /<section class="learn-sources"[\s\S]*?<\/section>/.exec(html)?.[0] ?? '';
    const cited = [...sources.matchAll(/<li><a href="(https:\/\/[^"]+)"/g)];
    if (cited.length < 2) problems.push(`${slug}: ${cited.length} sources, want at least 2 with https links`);
    if (!/<section class="learn-live"[\s\S]*?<astro-island/.test(html)) problems.push(`${slug}: no live example`);
    if (!/<h2[^>]*>[^<]+<\/h2>/.test(body)) problems.push(`${slug}: no section headings`);
  }
  assert.deepEqual(problems, []);
});

test('every explainer is linked from /learn/ and from each tool it names', () => {
  const hub = page('learn');
  const problems = [];
  for (const slug of slugs) {
    if (!hub.includes(`href="/learn/${slug}/"`)) problems.push(`/learn/ does not link ${slug}`);
    const html = page(`learn/${slug}`);
    const cards = /<section class="learn-tools"[\s\S]*?<\/section>/.exec(html)?.[0] ?? '';
    for (const [, href] of cards.matchAll(/<a href="(\/[^"]+\/)"/g)) {
      if (!existsSync(join(dist, href, 'index.html'))) { problems.push(`${slug}: ${href} is not a page`); continue; }
      if (!page(href).includes(`href="/learn/${slug}/"`)) problems.push(`${href} does not link back to ${slug}`);
    }
  }
  assert.deepEqual(problems, []);
});
