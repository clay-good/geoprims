#!/usr/bin/env node
// Post-build offline step (web/offline-pwa): lists every page, script, style,
// Wasm module, the catalog, and calculation data assets for the service worker
// to precache, stamps the release version into the pages, and writes dist/sw.js.
// Optional packs stay out of the install and are cached after use.
// Downloads (test vectors), share cards, crawler files, and docs pages (cached
// when visited) are left out. Fails the build if
// the precache exceeds 12 MB of Brotli-compressed transfer.
import { createHash } from 'node:crypto';
import { readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { brotliCompressSync } from 'node:zlib';

const web = new URL('..', import.meta.url).pathname;
const dist = join(web, 'dist');
// The budget lives in the reference profile, so one file holds every number
// a gate reports against (contracts/reference-profiles).
export const PROFILE = JSON.parse(readFileSync(join(web, '../../data/reference-profile.json'), 'utf8'));
export const PRECACHE_BUDGET = PROFILE.budgets.precacheBytes.hard;
export const VERSION_MARK = '__GP_APP_VERSION__';
const SKIP_DIRS = new Set(['vectors', 'sitemaps', '.well-known', 'og']);
// Docs pages are cached when visited, not installed (build-web-experience
// design: the precache is the shell, catalog, search index, Wasm, and tools).
export const DOCS_DIRS = ['learn', 'changelog', 'verification'];
const SKIP_FILES = new Set(['robots.txt', 'llms.txt', 'AGENTS.md', 'sitemap-index.xml', 'sw.js', '_headers', '_redirects']);

function files(dir, out = []) {
  for (const f of readdirSync(dir)) {
    const p = join(dir, f);
    if (statSync(p).isDirectory()) {
      if (!(dir === dist && SKIP_DIRS.has(f))) files(p, out);
    } else if (!(dir === dist && SKIP_FILES.has(f))) out.push(p);
  }
  return out;
}

const urlOf = (p) => '/' + relative(dist, p).replace(/(^|\/)index\.html$/, '$1');
const registry = JSON.parse(readFileSync(join(web, '../../assets/registry.json'), 'utf8'));
const optionalPackFiles = new Set(registry.assets
  .filter((asset) => asset.loadPolicy === 'on-demand' && asset.offlinePack)
  .flatMap((asset) => Object.keys(asset.files).map((file) => `/assets/${asset.id}/${asset.version}/${file}`)));
const packs = registry.assets
  .filter((asset) => asset.loadPolicy === 'on-demand' && asset.offlinePack)
  .map((asset) => ({
    id: asset.id,
    version: asset.version,
    title: asset.title,
    bytes: Object.values(asset.files).filter((file) => file.pack !== false).reduce((sum, file) => sum + file.bytes, 0),
    files: Object.entries(asset.files).filter(([, file]) => file.pack !== false).map(([name, file]) => ({
      path: `/assets/${asset.id}/${asset.version}/${name}`,
      bytes: file.bytes,
      sha256: file.sha256,
    })),
  }));
const isDocs = (u) => DOCS_DIRS.some((d) => u === `/${d}/` || u.startsWith(`/${d}/`));
const list = files(dist).filter((p) => !optionalPackFiles.has(urlOf(p)) && !isDocs(urlOf(p))).sort();

// The version names the release's content, computed before it is stamped in.
const h = createHash('sha256');
for (const p of list) h.update(urlOf(p)).update(createHash('sha256').update(readFileSync(p)).digest());
const version = h.digest('hex').slice(0, 12);

let compressed = 0;
for (const p of list) {
  let body = readFileSync(p);
  if (p.endsWith('.html')) {
    const html = body.toString('utf8');
    if (html.includes(VERSION_MARK)) writeFileSync(p, (body = Buffer.from(html.replaceAll(VERSION_MARK, version))));
  }
  compressed += brotliCompressSync(body).length;
}
if (compressed > PRECACHE_BUDGET) {
  throw new Error(`precache is ${(compressed / 1e6).toFixed(2)} MB compressed, over the ${PRECACHE_BUDGET / 1e6} MB budget`);
}

const urls = list.map(urlOf);
const source = readFileSync(join(web, 'sw/sw.js'), 'utf8');
writeFileSync(join(dist, 'sw.js'), `const VERSION = '${version}';\nconst PRECACHE = ${JSON.stringify(urls)};\nconst PACKS = ${JSON.stringify(packs)};\nconst DOCS = ${JSON.stringify(DOCS_DIRS.map((d) => `/${d}/`))};\n\n${source}`);
console.log(`pwa: sw.js version ${version}, ${urls.length} files, ${(compressed / 1e6).toFixed(2)} MB compressed (budget ${PRECACHE_BUDGET / 1e6} MB)`);
