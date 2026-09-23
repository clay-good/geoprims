// Share cards (discovery/search-pages, "Open Graph images"): every indexable
// page names a 1200 × 630 image that exists in the build, with alt text, and
// the build-only data-og-* attributes never ship.
import assert from 'node:assert/strict';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';

const dist = join(new URL('..', import.meta.url).pathname, 'dist');

function htmlFiles(dir, out = []) {
  for (const f of readdirSync(dir)) {
    const p = join(dir, f);
    if (statSync(p).isDirectory()) htmlFiles(p, out);
    else if (f.endsWith('.html')) out.push(p);
  }
  return out;
}

/** Width and height from a JPEG's start-of-frame marker. */
export function jpegSize(buf) {
  let i = 2;
  while (i < buf.length) {
    const marker = buf[i + 1];
    const len = buf.readUInt16BE(i + 2);
    if (marker >= 0xc0 && marker <= 0xc3) return [buf.readUInt16BE(i + 7), buf.readUInt16BE(i + 5)];
    i += 2 + len;
  }
  return null;
}

test('every indexable page has a 1200 × 630 share card with alt text', () => {
  const problems = [];
  let checked = 0;
  for (const f of htmlFiles(dist)) {
    const html = readFileSync(f, 'utf8');
    const name = f.slice(dist.length);
    if (/data-og-/.test(html)) problems.push(`${name}: build-only data-og-* attributes shipped`);
    if (/<meta name="robots" content="noindex"/.test(html)) continue;
    checked += 1;
    const img = /<meta property="og:image" content="https:\/\/geoprims\.com(\/og\/[^"]+\.jpg)">/.exec(html)?.[1];
    if (!img) { problems.push(`${name}: no og:image`); continue; }
    if (!/<meta property="og:image:alt" content="[^"]+">/.test(html)) problems.push(`${name}: no og:image:alt`);
    let size = null;
    try { size = jpegSize(readFileSync(join(dist, img))); } catch { problems.push(`${name}: ${img} is missing`); continue; }
    if (size?.join('x') !== '1200x630') problems.push(`${name}: ${img} is ${size?.join('x')}`);
  }
  assert.ok(checked > 200, `only ${checked} indexable pages were checked`);
  assert.deepEqual(problems, []);
});
