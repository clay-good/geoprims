import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { nodeHost } from '../../../../packages/runtime/src/node.mjs';

const root = fileURLToPath(new URL('../../../..', import.meta.url));
const web = join(root, 'apps/web');
const ids = [
  'units.speed.convert',
  'geodesy.utm.forward',
  'aviation.altimetry.density-altitude',
  'indexing.h3.polygon-to-cells',
  'geodesy.geoid.geoid-height',
];

test('Chromium and Node return byte-identical results from the same Wasm', { timeout: 30_000 }, async (t) => {
  const server = spawn(process.execPath, ['scripts/serve.mjs', '0'], { cwd: web });
  t.after(() => server.kill());
  const port = await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.once('exit', (code) => reject(new Error(`Site server exited: ${code}`)));
    server.stdout.on('data', (chunk) => {
      const match = /localhost:(\d+)/.exec(chunk.toString());
      if (match) resolve(Number(match[1]));
    });
  });

  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const page = await browser.newPage();
  await page.goto(`http://127.0.0.1:${port}/offline/`);
  const workerFile = readdirSync(join(web, 'dist/_astro')).find((name) => /^compute\.worker-.*\.js$/.test(name));
  assert.ok(workerFile, 'the build has no browser compute worker');
  const host = nodeHost(join(root, 'dist/wasm'));

  for (const id of ids) {
    const line = readFileSync(join(root, 'core/vectors', `${id}.jsonl`), 'utf8').split('\n').find(Boolean);
    const input = JSON.stringify(JSON.parse(line).input);
    const node = await host.invoke(id, input);
    const inBrowser = await page.evaluate(({ workerFile, id, input }) => new Promise((resolve, reject) => {
      const worker = new Worker(`/_astro/${workerFile}`, { type: 'module' });
      worker.onmessage = ({ data }) => { worker.terminate(); resolve(data.out); };
      worker.onerror = (event) => { worker.terminate(); reject(new Error(event.message)); };
      worker.postMessage({ seq: 1, method: 'invoke', args: [id, input] });
    }), { workerFile, id, input });
    assert.equal(inBrowser, node, `${id} differs between Chromium and Node`);
  }
});
