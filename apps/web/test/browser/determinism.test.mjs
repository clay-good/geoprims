import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { chromium, firefox, webkit } from 'playwright';
import { nodeHost } from '../../../../packages/runtime/src/node.mjs';
import { serveBuiltSite, web } from './site.mjs';

const root = fileURLToPath(new URL('../../../..', import.meta.url));
const vectorDir = join(root, 'core/vectors');
const vectors = readdirSync(vectorDir).filter((name) => name.endsWith('.jsonl')).flatMap((name) => {
  const id = name.slice(0, -'.jsonl'.length);
  return readFileSync(join(vectorDir, name), 'utf8').split('\n').filter(Boolean)
    .map((line) => JSON.parse(line)).filter((v) => !v.supersededBy)
    .map((v) => ({ id, vector: v.id, input: JSON.stringify(v.input) }));
});

test('every live golden vector serializes identically in Chromium, Firefox, WebKit, and Node', { timeout: 600_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const workerFile = readdirSync(join(web, 'dist/_astro')).find((name) => /^compute\.worker-.*\.js$/.test(name));
  assert.ok(workerFile, 'the build has no browser compute worker');
  const host = nodeHost(join(root, 'dist/wasm'));
  const expected = [];
  for (const v of vectors) expected.push(await host.invoke(v.id, v.input));
  t.diagnostic(`${vectors.length} live vectors computed in Node`);

  for (const engine of [chromium, firefox, webkit]) {
    const browser = await engine.launch({ headless: true });
    try {
      const page = await browser.newPage();
      page.setDefaultTimeout(120_000);
      await page.goto(`${origin}/offline/`);
      await page.evaluate((name) => { window.computeWorker = new Worker(`/_astro/${name}`, { type: 'module' }); }, workerFile);
      for (let start = 0; start < vectors.length; start += 20) {
        const batch = vectors.slice(start, start + 20);
        const actual = await page.evaluate(async (calls) => {
          const results = [];
          for (const call of calls) {
            results.push(await new Promise((resolve, reject) => {
              const worker = window.computeWorker;
              worker.onmessage = ({ data }) => resolve(data.out);
              worker.onerror = (event) => reject(new Error(event.message));
              worker.postMessage({ seq: 1, method: 'invoke', args: [call.id, call.input] });
            }));
          }
          return results;
        }, batch);
        for (let i = 0; i < batch.length; i++) {
          const v = batch[i];
          assert.equal(actual[i], expected[start + i], `${engine.name()}: ${v.id} ${v.vector}`);
        }
      }
      t.diagnostic(`${engine.name()}: ${vectors.length} byte-identical results`);
    } finally {
      await browser.close();
    }
  }
});
