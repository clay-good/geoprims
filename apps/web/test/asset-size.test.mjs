// Cloudflare Workers refuses to deploy a build containing any single asset over
// 25 MiB, and nothing checked it. Two files were over: the 49.5 MiB
// timezones.geojson.zip, which the registry marks `pack: false` and
// prepare-assets.mjs was copying anyway, and boundaries.bin at 26.9 MiB. The
// deploy aborted on the first file it met, so geoprims.com served a build from
// before 2026-10-12 for about 317 commits without anything going red.
//
// The PWA budget does not cover this: it measures the precache, which excludes
// both files.
//
// Both are fixed now: the archive is no longer served, and the index is split
// into pinned parts. This test is what keeps a third one from appearing.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';

const web = new URL('..', import.meta.url).pathname;
const dist = join(web, 'dist');

/** https://developers.cloudflare.com/workers/static-assets/#limitations */
const MAX_ASSET_BYTES = 25 * 1024 * 1024;

/**
 * Assets allowed over the limit. Empty, and meant to stay that way: the one
 * file that was over, the 26.9 MiB time-zone boundary index, is now served as
 * the parts `assets/registry.json` pins for it, which the runtime's asset
 * provider joins back into one verified buffer.
 */
const KNOWN_OVER = [];

function walk(dir, prefix = '') {
  const out = [];
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const rel = prefix ? `${prefix}/${e.name}` : e.name;
    if (e.isDirectory()) out.push(...walk(join(dir, e.name), rel));
    else out.push([rel, statSync(join(dir, e.name)).size]);
  }
  return out;
}

test('no new asset is over the size Cloudflare will deploy', () => {
  const over = walk(dist)
    .filter(([, bytes]) => bytes > MAX_ASSET_BYTES)
    .map(([rel, bytes]) => `${rel} is ${(bytes / 1048576).toFixed(1)} MiB, over the 25 MiB Cloudflare deploys`);
  const unexpected = over.filter((m) => !KNOWN_OVER.some((k) => m.startsWith(k)));
  assert.deepEqual(unexpected, [], 'an asset this size stops the whole site deploying, not just its own page');
});

test('the files the registry keeps for provenance are not served', () => {
  const served = new Set(walk(dist).map(([rel]) => rel));
  const registry = JSON.parse(readFileSync(join(dist, 'assets/registry.json'), 'utf8'));
  const problems = [];
  for (const a of registry.assets) {
    for (const [name, f] of Object.entries(a.files ?? {})) {
      const rel = `assets/${a.id}/${a.version}/${name}`;
      if (f.pack === false && served.has(rel)) {
        problems.push(`${rel} is marked pack: false but the build serves it`);
      }
    }
  }
  assert.ok(
    registry.assets.some((a) => Object.values(a.files ?? {}).some((f) => f.pack === false)),
    'no file is marked pack: false, so this test checks nothing',
  );
  assert.deepEqual(problems, []);
});
