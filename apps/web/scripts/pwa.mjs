#!/usr/bin/env node
// Post-build offline step (web/offline-pwa): lists every page, script, style,
// Wasm module, the catalog, and calculation data assets for the service worker
// to precache, stamps the release version into the pages, and writes dist/sw.js.
// Optional packs stay out of the install and are cached after use.
// Downloads (test vectors), share cards, crawler files, and docs pages (cached
// when visited) are left out. Fails the build if
// the precache exceeds the profile's Brotli-compressed transfer budget.
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
// A file the registry splits into `parts` is served as those parts, so it is
// the parts that reach dist and the parts the pack has to name. Naming the
// whole file instead left them looking like ordinary pages: they went into the
// precache, which is how a 14 MB budget came to report 30 MB.
const served = (name, file) => (Array.isArray(file.parts)
  ? file.parts.map((part) => ({ name: part.file, bytes: part.bytes, sha256: part.sha256 }))
  : [{ name, bytes: file.bytes, sha256: file.sha256 }]);
const optional = registry.assets.filter((asset) => asset.loadPolicy === 'on-demand' && asset.offlinePack);
const optionalPackFiles = new Set(optional.flatMap((asset) =>
  Object.entries(asset.files).flatMap(([name, file]) =>
    served(name, file).map((f) => `/assets/${asset.id}/${asset.version}/${f.name}`))));
const packs = optional.map((asset) => {
  const files = Object.entries(asset.files)
    .filter(([, file]) => file.pack !== false)
    .flatMap(([name, file]) => served(name, file).map((f) => ({
      path: `/assets/${asset.id}/${asset.version}/${f.name}`,
      bytes: f.bytes,
      sha256: f.sha256,
    })));
  return {
    id: asset.id,
    version: asset.version,
    title: asset.title,
    bytes: files.reduce((sum, f) => sum + f.bytes, 0),
    files,
  };
});
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
