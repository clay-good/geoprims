#!/usr/bin/env node
// IndexNow after a production deploy (add-seo-and-discoverability 3.3, search-pages
// "Sitemaps"): tells Bing and the other IndexNow engines which pages changed, so
// they recrawl them without waiting for the sitemap. Google does not use
// IndexNow; it reads the sitemap.
//
// The changed pages are the ones whose entry in the lastmod ledger
// (data/seo/lastmod.json) is new or has a new content hash since a git ref,
// normally the commit that was last deployed:
//
//   node scripts/indexnow.mjs --since <git ref>   # preview the changed URLs
//   node scripts/indexnow.mjs --all               # preview every indexable URL
//   node scripts/indexnow.mjs --since <ref> --send
//
// Without --send it only prints what it would submit. The key file it names is
// public/<key>.txt, served at the site root.
import { execFileSync } from 'node:child_process';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
export const SITE = 'https://geoprims.com';
export const ENDPOINT = 'https://api.indexnow.org/indexnow';
const LEDGER = 'data/seo/lastmod.json';

/** The key: the one public/<32 hex>.txt file whose contents are its own name. */
export function readKey(dir = join(web, 'public')) {
  const keys = readdirSync(dir)
    .filter((f) => /^[0-9a-f]{32}\.txt$/.test(f))
    .filter((f) => readFileSync(join(dir, f), 'utf8').trim() === f.slice(0, -4));
  if (keys.length !== 1) throw new Error(`expected one IndexNow key file in public/, found ${keys.length}`);
  return keys[0].slice(0, -4);
}

/** Paths that are new in `next` or whose content hash moved since `prev`, sorted. */
export function changedPaths(prev, next) {
  return Object.keys(next)
    .filter((p) => prev[p]?.hash !== next[p].hash)
    .sort();
}

/** The IndexNow request body for a set of site paths. */
export function payload(paths, key) {
  return {
    host: new URL(SITE).host,
    key,
    keyLocation: `${SITE}/${key}.txt`,
    urlList: paths.map((p) => SITE + p),
  };
}

function ledgerAt(ref) {
  try {
    return JSON.parse(execFileSync('git', ['show', `${ref}:${LEDGER}`], { cwd: root, encoding: 'utf8' }));
  } catch {
    throw new Error(`could not read ${LEDGER} at ${ref}`);
  }
}

async function main(argv) {
  const at = argv.indexOf('--since');
  const all = argv.includes('--all');
  if (!all && (at < 0 || !argv[at + 1])) {
    console.error('usage: node scripts/indexnow.mjs (--since <git ref> | --all) [--send]');
    process.exit(2);
  }
  const current = JSON.parse(readFileSync(join(root, LEDGER), 'utf8'));
  const paths = all ? Object.keys(current).sort() : changedPaths(ledgerAt(argv[at + 1]), current);
  const body = payload(paths, readKey());
  if (!paths.length) {
    console.log('indexnow: no changed pages');
    return;
  }
  if (!argv.includes('--send')) {
    console.log(body.urlList.join('\n'));
    console.log(`indexnow: ${paths.length} URLs would be submitted (preview only; add --send to submit)`);
    return;
  }
  // IndexNow takes up to 10,000 URLs a request; the site has a few hundred.
  const res = await fetch(ENDPOINT, {
    method: 'POST',
    headers: { 'content-type': 'application/json; charset=utf-8' },
    body: JSON.stringify(body),
  });
  // 200 and 202 both mean accepted; 202 while the key is still being checked.
  if (res.status !== 200 && res.status !== 202) {
    console.error(`indexnow: ${res.status} ${await res.text()}`);
    process.exit(1);
  }
  console.log(`indexnow: submitted ${paths.length} URLs (${res.status})`);
}

if (import.meta.url === `file://${process.argv[1]}`) await main(process.argv.slice(2));
