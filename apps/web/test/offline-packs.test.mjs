import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { formatBytes, isSafariTab, storageSentence } from '../src/lib/offline-packs.js';

const web = new URL('..', import.meta.url).pathname;

test('pack size comes from the registry and is shown on the manager', () => {
  const registry = JSON.parse(readFileSync(join(web, '../../assets/registry.json'), 'utf8'));
  const asset = registry.assets.find((item) => item.id === 'ne-50m');
  const bytes = Object.values(asset.files).reduce((sum, file) => sum + file.bytes, 0);
  assert.equal(bytes, 684709);
  assert.equal(formatBytes(bytes), '685 KB');
  const html = readFileSync(join(web, 'dist/offline/index.html'), 'utf8');
  assert.match(html, /Natural Earth 1:50m base map/);
  assert.match(html, /685 KB/);
  assert.match(html, /version 5\.1\.2/);
});

test('Safari tab warning applies only outside an installed app', () => {
  const safari = { userAgent: 'Mozilla/5.0 Version/18.0 Safari/605.1.15' };
  const chrome = { userAgent: 'Mozilla/5.0 Chrome/128.0.0.0 Safari/537.36' };
  assert.equal(isSafariTab(safari, () => ({ matches: false })), true);
  assert.equal(isSafariTab({ ...safari, standalone: true }, () => ({ matches: false })), false);
  assert.equal(isSafariTab(safari, () => ({ matches: true })), false);
  assert.equal(isSafariTab(chrome, () => ({ matches: false })), false);
});

test('storage usage and quota are stated in readable units', () => {
  assert.equal(storageSentence({ usage: 2_400_000, quota: 50_000_000 }), '2.4 MB used of 50.0 MB available to this site.');
  assert.equal(storageSentence({}), 'Storage use is unavailable in this browser.');
});
