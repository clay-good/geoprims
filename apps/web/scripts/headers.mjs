#!/usr/bin/env node
// Post-build security headers (define-build-contracts B2, platform
// privacy-and-security). Hashes every inline script and style Astro emits,
// writes the enumerated CSP and security headers to dist/_headers for Workers
// static assets, and puts the same CSP in a meta tag on every page for hosts
// that ignore _headers. Runs before scripts/pwa.mjs, which versions the pages.
import { createHash } from 'node:crypto';
import { readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const web = new URL('..', import.meta.url).pathname;
const dist = join(web, 'dist');

export const ASSET_ORIGIN = 'https://assets.geoprims.com';
export const TURNSTILE = 'https://challenges.cloudflare.com';

/** The CSP directives, in contract order, allowing the given inline hashes. */
export function policy({ scripts = [], styles = [] }) {
  const h = (list) => list.map((x) => `'sha256-${x}'`);
  return [
    ["default-src", "'self'"],
    ['script-src', "'self'", "'wasm-unsafe-eval'", TURNSTILE, ...h(scripts)],
    ['style-src', "'self'", ...h(styles)],
    ['img-src', "'self'", 'blob:', 'data:', ASSET_ORIGIN],
    ['connect-src', "'self'", ASSET_ORIGIN],
    ['worker-src', "'self'", 'blob:'],
    ['frame-src', TURNSTILE],
    ['font-src', "'self'"],
    ['object-src', "'none'"],
    ['base-uri', "'none'"],
    ['frame-ancestors', "'none'"],
    ['form-action', "'none'"],
    ['manifest-src', "'self'"],
  ];
}

export const serialize = (directives) => directives.map((d) => d.join(' ')).join('; ');

export const SECURITY_HEADERS = {
  'Referrer-Policy': 'no-referrer',
  'X-Content-Type-Options': 'nosniff',
  'Permissions-Policy': 'camera=(), microphone=(), payment=(), usb=(), geolocation=(self)',
  'Strict-Transport-Security': 'max-age=63072000; includeSubDomains; preload',
  'Cross-Origin-Opener-Policy': 'same-origin',
};

/** Inline scripts and styles (no src, no type other than JavaScript) and their SHA-256. */
export function inlineHashes(html) {
  const sha = (s) => createHash('sha256').update(s, 'utf8').digest('base64');
  const scripts = [...html.matchAll(/<script(?![^>]*\bsrc=)(?![^>]*\btype="application\/ld\+json")[^>]*>([\s\S]*?)<\/script>/g)].map((m) => sha(m[1]));
  const styles = [...html.matchAll(/<style[^>]*>([\s\S]*?)<\/style>/g)].map((m) => sha(m[1]));
  return { scripts, styles };
}

function htmlFiles(dir, out = []) {
  for (const f of readdirSync(dir)) {
    const p = join(dir, f);
    if (statSync(p).isDirectory()) htmlFiles(p, out);
    else if (f.endsWith('.html')) out.push(p);
  }
  return out;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const pages = htmlFiles(dist);
  const scripts = new Set();
  const styles = new Set();
  for (const p of pages) {
    const found = inlineHashes(readFileSync(p, 'utf8'));
    found.scripts.forEach((x) => scripts.add(x));
    found.styles.forEach((x) => styles.add(x));
  }
  const directives = policy({ scripts: [...scripts].sort(), styles: [...styles].sort() });
  const csp = serialize(directives);
  // A meta CSP cannot set frame-ancestors; the header does.
  const meta = serialize(directives.filter(([name]) => name !== 'frame-ancestors'));
  for (const p of pages) {
    const html = readFileSync(p, 'utf8');
    if (html.includes('http-equiv="Content-Security-Policy"')) continue;
    writeFileSync(p, html.replace('<meta charset="utf-8">', `<meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="${meta}">`));
  }
  const block = (path, headers) => `${path}\n${Object.entries(headers).map(([k, v]) => `  ${k}: ${v}`).join('\n')}\n`;
  writeFileSync(
    join(dist, '_headers'),
    [
      block('/*', { 'Content-Security-Policy': csp, ...SECURITY_HEADERS }),
      block('/_astro/*', { 'Cache-Control': 'public, max-age=31536000, immutable' }),
      block('/sw.js', { 'Cache-Control': 'no-cache' }),
      block('/manifest.webmanifest', { 'Cache-Control': 'no-cache' }),
    ].join('\n'),
  );
  console.log(`headers: _headers and meta CSP on ${pages.length} pages (${scripts.size} inline scripts, ${styles.size} inline styles by hash)`);
}
