#!/usr/bin/env node
// Verify that a deployed static build serves the same security and cache
// headers written to dist/_headers. Preview deployments must also stay out of
// search indexes.
import { readdirSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { matches, parseHeaders } from './serve.mjs';

const web = fileURLToPath(new URL('..', import.meta.url));
const dist = join(web, 'dist');

function expectedFor(rules, path) {
  return Object.assign({}, ...rules.filter(([pattern]) => matches(pattern, path)).map(([, headers]) => headers));
}

export async function smoke(origin, fetchImpl = fetch) {
  const base = new URL(origin);
  if (base.protocol !== 'https:') throw new Error('preview origin must use HTTPS');
  const asset = readdirSync(join(dist, '_astro')).find((name) => name.endsWith('.js'));
  if (!asset) throw new Error('the built site has no hashed JavaScript asset');
  const checks = [
    ['/', 200, true],
    ['/aviation/altimetry/density-altitude/', 200, true],
    ['/wasm/base.wasm', 200, false],
    [`/_astro/${asset}`, 200, false],
    ['/sw.js', 200, false],
    ['/not-a-real-route', 404, true],
  ];
  const rules = parseHeaders(readFileSync(join(dist, '_headers'), 'utf8'));
  for (const [path, status, html] of checks) {
    const response = await fetchImpl(new URL(path, base));
    if (response.status !== status) throw new Error(`${path}: expected ${status}, got ${response.status}`);
    const expected = expectedFor(rules, path);
    for (const name of ['Content-Security-Policy', 'Referrer-Policy', 'X-Content-Type-Options', 'Permissions-Policy', 'Strict-Transport-Security', 'Cross-Origin-Opener-Policy']) {
      if (response.headers.get(name) !== expected[name]) throw new Error(`${path}: ${name} differs from dist/_headers`);
    }
    if (expected['Cache-Control'] && response.headers.get('Cache-Control') !== expected['Cache-Control']) {
      throw new Error(`${path}: Cache-Control differs from dist/_headers`);
    }
    if (response.headers.get('X-Robots-Tag') !== 'noindex, nofollow, noarchive') {
      throw new Error(`${path}: preview is missing X-Robots-Tag`);
    }
    if (response.headers.has('Set-Cookie')) throw new Error(`${path}: response sets a cookie`);
    if (html) {
      const body = await response.text();
      const meta = expected['Content-Security-Policy'].split('; ').filter((directive) => !directive.startsWith('frame-ancestors')).join('; ');
      if (!body.includes(`<meta http-equiv="Content-Security-Policy" content="${meta}">`)) {
        throw new Error(`${path}: meta CSP differs from the response header`);
      }
    }
  }
  return checks.map(([path]) => path);
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  const origin = process.argv[2];
  if (!origin) {
    console.error('usage: node scripts/header-smoke.mjs https://preview.example');
    process.exitCode = 2;
  } else {
    smoke(origin)
      .then((paths) => console.log(`header smoke: ${paths.length} preview routes passed`))
      .catch((error) => { console.error(error.message); process.exitCode = 1; });
  }
}
