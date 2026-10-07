// IndexNow submission after a deploy (add-seo-and-discoverability 3.3): only
// pages whose words changed are sent, and the key the request names is the
// key file the site serves.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { changedPaths, payload, readKey, SITE } from '../scripts/indexnow.mjs';

const web = new URL('..', import.meta.url).pathname;

test('only new pages and pages whose content hash moved are submitted', () => {
  const prev = { '/a/': { hash: '1', lastmod: '2026-09-01' }, '/b/': { hash: '2', lastmod: '2026-09-01' }, '/gone/': { hash: '9', lastmod: '2026-09-01' } };
  const next = { '/a/': { hash: '1', lastmod: '2026-09-01' }, '/b/': { hash: '3', lastmod: '2026-10-07' }, '/c/': { hash: '4', lastmod: '2026-10-07' } };
  assert.deepEqual(changedPaths(prev, next), ['/b/', '/c/']);
  assert.deepEqual(changedPaths(next, next), []);
});

test('the request names the key file the site serves, and absolute URLs on the site', () => {
  const key = readKey();
  assert.match(key, /^[0-9a-f]{32}$/);
  assert.equal(readFileSync(join(web, 'public', `${key}.txt`), 'utf8').trim(), key);
  const body = payload(['/', '/aviation/'], key);
  assert.deepEqual(body, {
    host: 'geoprims.com',
    key,
    keyLocation: `${SITE}/${key}.txt`,
    urlList: [`${SITE}/`, `${SITE}/aviation/`],
  });
});
