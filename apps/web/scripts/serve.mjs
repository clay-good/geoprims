#!/usr/bin/env node
// Serves dist/ locally with the headers from dist/_headers applied, so pages
// run under the production CSP (the Astro preview server ignores _headers).
// Usage: node scripts/serve.mjs [port]
import { createServer } from 'node:http';
import { existsSync, readFileSync, statSync } from 'node:fs';
import { extname, join, normalize } from 'node:path';

const dist = join(new URL('..', import.meta.url).pathname, 'dist');
const port = Number(process.argv[2] ?? 4322);
const TYPES = {
  '.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json',
  '.wasm': 'application/wasm', '.svg': 'image/svg+xml', '.png': 'image/png', '.webmanifest': 'application/manifest+json',
  '.txt': 'text/plain; charset=utf-8', '.xml': 'application/xml', '.jsonl': 'application/jsonl', '.md': 'text/markdown',
};

/** Parses _headers into [pattern, headers] rules, in file order. */
export function parseHeaders(text) {
  const rules = [];
  for (const line of text.split('\n')) {
    if (!line.trim()) continue;
    if (!line.startsWith(' ')) rules.push([line.trim(), {}]);
    else {
      const i = line.indexOf(':');
      rules.at(-1)[1][line.slice(0, i).trim()] = line.slice(i + 1).trim();
    }
  }
  return rules;
}

export const matches = (pattern, path) =>
  new RegExp(`^${pattern.replace(/[.+?^${}()|[\]\\]/g, '\\$&').replaceAll('*', '.*')}$`).test(path);

if (import.meta.url === `file://${process.argv[1]}`) {
  const rules = parseHeaders(readFileSync(join(dist, '_headers'), 'utf8'));
  createServer((req, res) => {
    const path = decodeURIComponent(new URL(req.url, 'http://x').pathname);
    let file = normalize(join(dist, path));
    if (!file.startsWith(dist)) return res.writeHead(403).end();
    if (existsSync(file) && statSync(file).isDirectory()) file = join(file, 'index.html');
    // Unknown URLs get the site's own not-found page, as static hosts serve 404.html.
    let status = 200;
    if (!existsSync(file)) {
      if (!existsSync(join(dist, '404.html'))) return res.writeHead(404).end('not found');
      file = join(dist, '404.html');
      status = 404;
    }
    for (const [pattern, headers] of rules) if (matches(pattern, path)) for (const [k, v] of Object.entries(headers)) res.setHeader(k, v);
    res.setHeader('Content-Type', TYPES[extname(file)] ?? 'application/octet-stream');
    res.writeHead(status).end(readFileSync(file));
  }).listen(port, function () { console.log(`serving dist with _headers on http://localhost:${this.address().port}`); });
}
