#!/usr/bin/env node
// Post-build link safety (trust/citations, "Bare links are safe and specific").
// Every link that leaves the site gets rel="noopener noreferrer", so a page we
// link to cannot reach back through window.opener and nothing about the
// reader's path travels with the request. Doing it here rather than in each
// template means a new link cannot forget it.
import { readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const dist = join(new URL('..', import.meta.url).pathname, 'dist');
const SAFE = 'noopener noreferrer';

/** Every anchor in `html` that points off-site, with rel added or merged. */
export function safeLinks(html) {
  let added = 0;
  const out = html.replace(/<a\s([^>]*href="https?:\/\/[^"]+"[^>]*)>/g, (tag, attrs) => {
    const rel = /\brel="([^"]*)"/.exec(attrs);
    if (rel) {
      const parts = new Set(rel[1].split(/\s+/).filter(Boolean));
      if (parts.has('noopener') && parts.has('noreferrer')) return tag;
      for (const p of SAFE.split(' ')) parts.add(p);
      added += 1;
      return `<a ${attrs.replace(rel[0], `rel="${[...parts].join(' ')}"`)}>`;
    }
    added += 1;
    return `<a ${attrs} rel="${SAFE}">`;
  });
  return { html: out, added };
}

/** Anchors that still leave the site without the rel, for the gate. */
export const unsafeLinks = (html) =>
  [...html.matchAll(/<a\s[^>]*href="https?:\/\/[^"]+"[^>]*>/g)]
    .filter((m) => !/\brel="[^"]*noopener[^"]*"/.test(m[0]) || !/\brel="[^"]*noreferrer[^"]*"/.test(m[0]))
    .map((m) => m[0]);

const pages = [];
(function walk(dir) {
  for (const f of readdirSync(dir)) {
    const p = join(dir, f);
    if (statSync(p).isDirectory()) walk(p);
    else if (f.endsWith('.html')) pages.push(p);
  }
})(dist);

let changed = 0;
let links = 0;
for (const p of pages) {
  const { html, added } = safeLinks(readFileSync(p, 'utf8'));
  if (added) {
    writeFileSync(p, html);
    changed += 1;
    links += added;
  }
  const left = unsafeLinks(readFileSync(p, 'utf8'));
  if (left.length) throw new Error(`${p}: ${left.length} link(s) still unsafe, first ${left[0]}`);
}
console.log(`links: ${links} outbound links made safe across ${changed} of ${pages.length} pages`);
