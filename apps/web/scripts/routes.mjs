#!/usr/bin/env node
// Post-build route-map gate (define-build-contracts, contracts/routes-and-urls).
// Classifies every HTML route the build emitted against the contract's closed
// route map and fails naming anything outside it, so a stray page can never
// ship. Non-indexable pages must say so: a generated endpoint canonicalizes to
// the operation it composes, and app routes carry noindex. Also writes
// dist/_redirects from data/redirects.json, the file that keeps a renamed
// tool's old route working for at least 24 months.
import { readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { buildDate } from '../src/lib/build-date.mjs';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const dist = join(web, 'dist');

export const SITE = 'https://geoprims.com';

/** Routes that stand alone, each with the class it belongs to. */
export const FIXED = new Map([
  ['/', 'home'],
  ['/tools/', 'catalog'],
  ['/agents/', 'agents'],
  ['/404.html', 'not-found'],
  ['/sources/', 'trust'],
  ['/methodology/', 'trust'],
  ['/verification/', 'trust'],
  ['/changelog/', 'trust'],
  ['/known-issues/', 'trust'],
  ['/disclaimer/', 'trust'],
  ['/quality/', 'trust'],
  ['/licenses/', 'trust'],
  ['/privacy/', 'trust'],
  ['/settings/', 'app'],
  ['/offline/', 'app'],
]);

/** Classes whose pages carry noindex instead of a canonical to a parent. */
export const NOINDEX_CLASSES = new Set(['app', 'not-found']);

/** Stable URL rules: lowercase, hyphenated segments, trailing slash. */
const STYLE = /^\/(?:[a-z0-9]+(?:-[a-z0-9]+)*\/)*$/;
const VERSION_ROUTE = /^\/verification\/\d+\.\d+\.\d+\/$/;

/** The domains, groups, and tool ids the catalog gives pages to. */
export function index(catalog) {
  const domains = new Set();
  const groups = new Set();
  const tools = new Map();
  for (const t of catalog.tools) {
    const [domain, group] = t.id.split('.');
    domains.add(domain);
    groups.add(`${domain}/${group}`);
    tools.set(t.id.split('.').join('/'), t);
  }
  return { domains, groups, tools };
}

/**
 * The route class of a built path, or null when it falls outside the map.
 * `endpoint` is a generated pair page; `tool` is a plain operation.
 */
export function classify(route, idx) {
  if (FIXED.has(route)) return FIXED.get(route);
  if (VERSION_ROUTE.test(route)) return 'verification';
  if (!STYLE.test(route)) return null;
  if (/^\/learn\/[a-z0-9-]+\/$/.test(route)) return 'explainer';
  if (/^\/journeys\/[a-z0-9-]+\/$/.test(route)) return 'journey';
  const path = route.slice(1, -1);
  const depth = path.split('/').length;
  if (depth === 1 && idx.domains.has(path)) return 'domain';
  if (depth === 2 && idx.groups.has(path)) return 'group';
  const t = idx.tools.get(path);
  if (t) return t.composedOf.length > 0 ? 'endpoint' : 'tool';
  return null;
}

const canonicalOf = (html) => /<link rel="canonical" href="([^"]*)"/.exec(html)?.[1];
const isNoindex = (html) => /<meta name="robots" content="noindex">/.test(html);

/** Every problem with the built routes, in the order the pages were listed. */
export function check(pages, idx) {
  const problems = [];
  for (const { route, html } of pages) {
    const kind = classify(route, idx);
    if (!kind) {
      problems.push(`${route} is not a route the contract's route map allows`);
      continue;
    }
    if (NOINDEX_CLASSES.has(kind)) {
      if (!isNoindex(html)) problems.push(`${route} is an app route and must carry noindex`);
      continue;
    }
    const canonical = canonicalOf(html);
    if (!canonical) {
      problems.push(`${route} has no canonical link`);
    } else if (kind === 'endpoint') {
      const parent = `${SITE}/${idx.tools.get(route.slice(1, -1)).composedOf[0].split('.').join('/')}/`;
      if (canonical !== parent) problems.push(`${route} is a generated endpoint and must canonicalize to ${parent}, not ${canonical}`);
    }
  }
  return problems;
}

/**
 * The redirects a deprecated tool implies: its own route points at the
 * replacement from the day it is deprecated, so a link keeps working when the
 * tool is finally removed (platform/tool-contract, "Deprecated id resolves to
 * replacement").
 */
export function deprecationRedirects(catalog, today = buildDate()) {
  return catalog.tools
    .filter((t) => t.deprecation)
    .map((t) => ({
      from: `/${t.id.split('.').join('/')}/`,
      to: `/${t.deprecation.replacement.split('.').join('/')}/`,
      since: today,
      deprecated: true,
    }))
    .sort((a, b) => a.from.localeCompare(b.from));
}

/** Problems with the redirects file itself (renamed routes must still resolve). */
export function checkRedirects(list, builtRoutes) {
  const problems = [];
  for (const { from, to, since, deprecated } of list) {
    if (!STYLE.test(from ?? '')) problems.push(`redirect from ${from} is not a well-formed route`);
    // A deprecated tool keeps its page until removal: the redirect is the
    // fallback for after it goes, so it may name a route the build still emits.
    else if (builtRoutes.has(from) && !deprecated) problems.push(`redirect from ${from} shadows a page the build still emits`);
    if (!builtRoutes.has(to)) problems.push(`redirect to ${to} is not a page this build emits`);
    if (!/^\d{4}-\d{2}-\d{2}$/.test(since ?? '')) problems.push(`redirect from ${from} has no ISO since date`);
  }
  return problems;
}

/** The Cloudflare static-assets redirects file. */
export const serialize = (list) => list.map(({ from, to }) => `${from} ${to} 301`).join('\n') + '\n';

function htmlFiles(dir, out = []) {
  for (const f of readdirSync(dir)) {
    const p = join(dir, f);
    if (statSync(p).isDirectory()) htmlFiles(p, out);
    else if (f.endsWith('.html')) out.push(p);
  }
  return out;
}

/** A built file path as the route it serves ("dist/a/index.html" is "/a/"). */
export const routeOf = (path) => path.slice(dist.length).replace(/\/index\.html$/, '/');

if (import.meta.url === `file://${process.argv[1]}`) {
  const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
  const idx = index(catalog);
  const pages = htmlFiles(dist)
    .sort()
    .map((p) => ({ route: routeOf(p), html: readFileSync(p, 'utf8') }));
  const list = [
    ...JSON.parse(readFileSync(join(root, 'data/redirects.json'), 'utf8')),
    ...deprecationRedirects(catalog),
  ];
  const problems = [...check(pages, idx), ...checkRedirects(list, new Set(pages.map((p) => p.route)))];
  if (problems.length > 0) {
    console.error(`routes: ${problems.length} problem(s)\n${problems.map((m) => `  ${m}`).join('\n')}`);
    process.exit(1);
  }
  writeFileSync(join(dist, '_redirects'), serialize(list));
  const counts = new Map();
  for (const { route } of pages) {
    const kind = classify(route, idx);
    counts.set(kind, (counts.get(kind) ?? 0) + 1);
  }
  const summary = [...counts].sort().map(([k, n]) => `${n} ${k}`).join(', ');
  console.log(`routes: ${pages.length} pages in the route map (${summary}), ${list.length} redirects`);
}
