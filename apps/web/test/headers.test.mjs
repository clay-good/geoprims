// Header smoke test (define-build-contracts B2, platform privacy-and-security
// "Header check"): the built _headers and meta CSP match the enumerated
// policy, every inline script and style is allowed by hash, and nothing
// needs 'unsafe-inline'.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { inlineHashes, SECURITY_HEADERS } from '../scripts/headers.mjs';
import { matches, parseHeaders } from '../scripts/serve.mjs';

const web = new URL('..', import.meta.url).pathname;
const dist = join(web, 'dist');
const rules = parseHeaders(readFileSync(join(dist, '_headers'), 'utf8'));
const headersFor = (path) => Object.assign({}, ...rules.filter(([p]) => matches(p, path)).map(([, h]) => h));
const directives = (csp) => Object.fromEntries(csp.split(';').map((d) => d.trim().split(/\s+/)).map(([k, ...v]) => [k, v]));

function htmlFiles(dir, out = []) {
  for (const f of readdirSync(dir)) {
    const p = join(dir, f);
    if (statSync(p).isDirectory()) htmlFiles(p, out);
    else if (f.endsWith('.html')) out.push(p);
  }
  return out;
}

// The contract's policy, without the build's inline hashes.
const REQUIRED = {
  'default-src': ["'self'"],
  'script-src': ["'self'", "'wasm-unsafe-eval'", 'https://challenges.cloudflare.com'],
  'style-src': ["'self'"],
  'img-src': ["'self'", 'blob:', 'data:', 'https://assets.geoprims.com'],
  'connect-src': ["'self'", 'https://assets.geoprims.com'],
  'worker-src': ["'self'", 'blob:'],
  'frame-src': ['https://challenges.cloudflare.com'],
  'font-src': ["'self'"],
  'object-src': ["'none'"],
  'base-uri': ["'none'"],
  'frame-ancestors': ["'none'"],
  'form-action': ["'none'"],
  'manifest-src': ["'self'"],
};

test('every route gets the enumerated CSP and security headers', () => {
  for (const path of ['/', '/aviation/altimetry/density-altitude/', '/methodology/', '/wasm/base.wasm']) {
    const h = headersFor(path);
    const csp = directives(h['Content-Security-Policy']);
    for (const [k, v] of Object.entries(REQUIRED)) {
      const rest = (csp[k] ?? []).filter((x) => !x.startsWith("'sha256-"));
      assert.deepEqual(rest, v, `${path} ${k}`);
    }
    assert.deepEqual(Object.keys(csp).sort(), Object.keys(REQUIRED).sort(), 'no extra directives');
    assert.ok(!/unsafe-inline|'unsafe-eval'/.test(h['Content-Security-Policy']));
    for (const [k, v] of Object.entries(SECURITY_HEADERS)) assert.equal(h[k], v, `${path} ${k}`);
  }
  assert.equal(headersFor('/_astro/app.abc123.js')['Cache-Control'], 'public, max-age=31536000, immutable');
  assert.equal(headersFor('/sw.js')['Cache-Control'], 'no-cache');
  assert.equal(headersFor('/').hasOwnProperty('Cache-Control'), false, 'pages are not cached as immutable');
});

test('every inline script and style is allowed by hash, and pages carry the meta CSP', () => {
  const csp = directives(headersFor('/')['Content-Security-Policy']);
  const meta = headersFor('/')['Content-Security-Policy'].split('; ').filter((d) => !d.startsWith('frame-ancestors')).join('; ');
  for (const f of htmlFiles(dist)) {
    const html = readFileSync(f, 'utf8');
    const where = f.slice(dist.length);
    const { scripts, styles } = inlineHashes(html);
    for (const x of scripts) assert.ok(csp['script-src'].includes(`'sha256-${x}'`), `${where}: inline script ${x} is not allowed`);
    for (const x of styles) assert.ok(csp['style-src'].includes(`'sha256-${x}'`), `${where}: inline style ${x} is not allowed`);
    assert.doesNotMatch(html, /\sstyle="/, `${where}: inline style attribute`);
    assert.doesNotMatch(html, /\son[a-z]+="/, `${where}: inline event handler`);
    assert.ok(html.includes(`<meta http-equiv="Content-Security-Policy" content="${meta}">`), `${where}: meta CSP`);
  }
});

test('an unhashed inline script is caught', () => {
  const csp = directives(headersFor('/')['Content-Security-Policy']);
  const [x] = inlineHashes('<script>alert(1)</script><script type="application/ld+json">{}</script><script src="/a.js"></script>').scripts;
  assert.equal(inlineHashes('<script>alert(1)</script>').scripts.length, 1, 'JSON-LD and external scripts are not hashed');
  assert.ok(!csp['script-src'].includes(`'sha256-${x}'`));
});
