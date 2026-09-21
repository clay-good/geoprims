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
