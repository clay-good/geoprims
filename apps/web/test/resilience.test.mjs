// app-shell "Error resilience" and "JavaScript disabled".
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const web = new URL('..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(web, '../../dist/catalog/v1.json'), 'utf8'));
const route = (id) => '/' + id.split('.').join('/') + '/';

test('with WebAssembly disabled, the calculator says why and what still works', () => {
  const worker = new URL('../src/lib/compute.worker.js', import.meta.url).href;
  const script = `
    const out = [];
    globalThis.self = { postMessage: (m) => out.push(m) };
    delete globalThis.WebAssembly;
    await import(${JSON.stringify(worker)});
    await self.onmessage({ data: { seq: 1, method: 'invoke', args: ['units.speed.kt-to-mph', '{"value":1}'] } });
    process.stdout.write(out[0].out);`;
  const r = JSON.parse(execFileSync(process.execPath, ['--input-type=module', '-e', script], { encoding: 'utf8' }));
  assert.equal(r.error.code, 'UNSUPPORTED');
  assert.match(r.error.message, /^This tool needs WebAssembly, which is disabled in this browser\./);
  assert.match(r.error.message, /documentation and worked example on this page still apply/);
});

test('with JavaScript disabled, every tool page explains and keeps its worked example and docs', () => {
  for (const t of catalog.tools) {
    const html = readFileSync(join(web, 'dist', route(t.id), 'index.html'), 'utf8');
    const i = html.indexOf('<noscript>');
    assert.ok(i > 0 && html.slice(i, i + 200).includes('Computing your own values needs JavaScript and WebAssembly'), t.id);
    assert.match(html, /class="sentence[^"]*">[^<]+</, `${t.id}: pre-rendered example`);
    assert.match(html, /<summary>How we got this/, `${t.id}: docs`);
  }
});

test('a missing data file is named, and the reader is told what to do', async () => {
  // web/offline-pwa "Missing asset offline": the dataset by name, whether the
  // reader is offline, and the fix — never an asset id and a file name.
  const { assetMessage } = await import('../src/lib/messages.js');
  const registry = JSON.parse(readFileSync(join(web, 'dist/assets/registry.json'), 'utf8'));
  const error = { code: 'ASSET_UNAVAILABLE', message: 'This needs the egm96-15 data (2009-08-29, egm96-15.pgm), which is not loaded yet.', asset: { id: 'egm96-15', version: '2009-08-29', key: 'egm96-15.pgm' } };
  const offline = assetMessage(error, registry, false);
  assert.match(offline.message, /EGM96 geoid, 15-minute grid/);
  assert.match(offline.message, /not available offline/);
  assert.match(offline.hint, /Connect once/);
  assert.doesNotMatch(offline.message + offline.hint, /egm96-15|\.pgm/, 'a file name reached the reader');
  const online = assetMessage(error, registry, true);
  assert.match(online.hint, /Reload/);
  const damaged = assetMessage({ ...error, code: 'ASSET_INTEGRITY' }, registry, true);
  assert.match(damaged.message, /failed its integrity check/);
  // An unknown asset still reads as words.
  assert.match(assetMessage({ code: 'ASSET_UNAVAILABLE', asset: { id: 'nope' } }, registry).message, /reference data/);
});
