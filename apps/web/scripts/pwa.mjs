#!/usr/bin/env node
// Post-build offline step (web/offline-pwa): lists every page, script, style,
// Wasm module, the catalog, and the data assets for the service worker to
// precache, stamps the release version into the pages, and writes dist/sw.js.
// Downloads (test vectors) and crawler files are left out. Fails the build if
// the precache exceeds 12 MB of Brotli-compressed transfer.
import { createHash } from 'node:crypto';
import { readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { brotliCompressSync } from 'node:zlib';

const web = new URL('..', import.meta.url).pathname;
const dist = join(web, 'dist');
export const PRECACHE_BUDGET = 12_000_000;
export const VERSION_MARK = '__GP_APP_VERSION__';
const SKIP_DIRS = new Set(['vectors', 'sitemaps', '.well-known']);
const SKIP_FILES = new Set(['robots.txt', 'llms.txt', 'AGENTS.md', 'sitemap-index.xml', 'sw.js', '_headers']);

function files(dir, out = []) {
  for (const f of readdirSync(dir)) {
    const p = join(dir, f);
    if (statSync(p).isDirectory()) {
      if (!(dir === dist && SKIP_DIRS.has(f))) files(p, out);
    } else if (!(dir === dist && SKIP_FILES.has(f))) out.push(p);
  }
  return out;
}

const list = files(dist).sort();
const urlOf = (p) => '/' + relative(dist, p).replace(/(^|\/)index\.html$/, '$1');

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
writeFileSync(join(dist, 'sw.js'), `const VERSION = '${version}';\nconst PRECACHE = ${JSON.stringify(urls)};\n\n${source}`);
console.log(`pwa: sw.js version ${version}, ${urls.length} files, ${(compressed / 1e6).toFixed(2)} MB compressed (budget ${PRECACHE_BUDGET / 1e6} MB)`);
