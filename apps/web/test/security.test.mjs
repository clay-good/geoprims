// The security contact (RFC 9116) and the third-party request audit
// (platform/privacy-and-security). No page may cause a request to an origin
// this site does not control, and the security contact must not be expired.
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const dist = join(web, 'dist');

/** The only external origin this site may reach, and only inside the report dialog. */
const ALLOWED_ORIGINS = new Set(['challenges.cloudflare.com', 'assets.geoprims.com']);

/** Attributes the browser fetches from. `a href` is a navigation, not a request. */
const REQUESTING = /<(?:script|link|img|iframe|video|audio|source|embed|object|track)\b[^>]*?\b(?:src|href|data|srcset|poster)="([^"]*)"/g;

function files(dir, out = [], match = () => true) {
  for (const f of readdirSync(dir)) {
    const p = join(dir, f);
    if (statSync(p).isDirectory()) files(p, out, match);
    else if (match(f)) out.push(p);
  }
  return out;
}

/** The external origins a document or stylesheet would make the browser reach. */
export function externalOrigins(text, { css = false } = {}) {
  const found = new Set();
  const add = (url) => {
    const m = /^(?:https?:)?\/\/([^/"')\s]+)/.exec(url.trim());
    if (m && !ALLOWED_ORIGINS.has(m[1])) found.add(m[1]);
  };
  if (css) {
    for (const m of text.matchAll(/url\(\s*['"]?([^'")]+)/g)) add(m[1]);
    for (const m of text.matchAll(/@import\s+(?:url\()?\s*['"]([^'"]+)/g)) add(m[1]);
    return found;
  }
  for (const m of text.matchAll(REQUESTING)) {
    if (/rel="canonical"/.test(m[0])) continue;
    add(m[1]);
  }
  for (const m of text.matchAll(/<style[^>]*>([\s\S]*?)<\/style>/g)) {
    for (const o of externalOrigins(m[1], { css: true })) found.add(o);
  }
  return found;
}

test('no built page makes the browser reach another origin', () => {
  const problems = [];
  for (const f of files(dist, [], (n) => n.endsWith('.html'))) {
    for (const o of externalOrigins(readFileSync(f, 'utf8'))) problems.push(`${f.slice(dist.length)} loads ${o}`);
  }
  assert.deepEqual(problems, []);
});

test('no stylesheet pulls in a font or image from another origin', () => {
  const problems = [];
  for (const f of files(dist, [], (n) => n.endsWith('.css'))) {
    for (const o of externalOrigins(readFileSync(f, 'utf8'), { css: true })) problems.push(`${f.slice(dist.length)} loads ${o}`);
  }
  assert.deepEqual(problems, []);
});

test('the audit catches an injected external font and script', () => {
  assert.deepEqual([...externalOrigins('<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Inter">')], ['fonts.googleapis.com']);
  assert.deepEqual([...externalOrigins('<style>@import url("https://fonts.gstatic.com/x.css");</style>')], ['fonts.gstatic.com']);
  assert.deepEqual([...externalOrigins('@font-face{src:url(//cdn.example.com/a.woff2)}', { css: true })], ['cdn.example.com']);
  assert.deepEqual([...externalOrigins('<img src="https://tracker.example/pixel.gif">')], ['tracker.example']);
  assert.deepEqual([...externalOrigins('<a href="https://github.com/clay-good/geoprims">Source</a>')], [], 'a link is not a request');
  assert.deepEqual([...externalOrigins('<link rel="canonical" href="https://geoprims.com/x/">')], [], 'the canonical is not a request');
  assert.deepEqual([...externalOrigins('<script src="https://challenges.cloudflare.com/turnstile/v0/api.js"></script>')], [], 'the bot check is allowed');
});

test('the security contact is published, complete, and not expiring soon', () => {
  const text = readFileSync(join(dist, '.well-known/security.txt'), 'utf8');
  const field = (name) => new RegExp(`^${name}: (.+)$`, 'm').exec(text)?.[1];
  // RFC 9116 requires Contact and Expires; Canonical must name this file.
  assert.match(field('Contact') ?? '', /^https:\/\//, 'Contact must be a URI');
  assert.equal(field('Canonical'), 'https://geoprims.com/.well-known/security.txt');
  assert.ok(field('Policy'), 'Policy points at SECURITY.md');
  const expires = Date.parse(field('Expires') ?? '');
  assert.ok(Number.isFinite(expires), 'Expires must be a date-time');
  const daysLeft = (expires - Date.now()) / 86_400_000;
  assert.ok(daysLeft > 30, `security.txt expires in ${Math.round(daysLeft)} days: renew it`);
  assert.ok(daysLeft < 366, 'RFC 9116 asks for less than a year');
});

test('SECURITY.md says where to report and what is out of scope', () => {
  const md = readFileSync(join(root, 'SECURITY.md'), 'utf8');
  assert.match(md, /security\/advisories\/new/);
  assert.match(md, /not in scope/i);
  assert.match(md, /A wrong answer is not a vulnerability/);
});
