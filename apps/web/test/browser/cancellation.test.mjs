import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

const web = fileURLToPath(new URL('../..', import.meta.url));

test('a long H3 calculation stops within 100 ms of an edit', { timeout: 30_000 }, async (t) => {
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
  await page.addInitScript(() => {
    window.workerStops = [];
    const terminate = Worker.prototype.terminate;
    Worker.prototype.terminate = function () {
      window.workerStops.push(performance.now());
      return terminate.call(this);
    };
  });
  await page.goto(`http://127.0.0.1:${port}/indexing/h3/polygon-to-cells/`);

  const points = page.locator('#field-points');
  await page.locator('#field-resolution').fill('10');
  await points.fill('40.0, -80.35\n40.0, -79.55\n40.5, -79.55\n40.5, -80.35');
  await page.getByText('Calculating for', { exact: false }).waitFor({ timeout: 15_000 });

  const latency = await page.evaluate(() => {
    const field = document.querySelector('#field-points');
    field.value = '40.4406, -80.002\n40.4406, -79.99\n40.448, -79.99\n40.448, -80.002';
    const editedAt = performance.now();
    field.dispatchEvent(new Event('input', { bubbles: true }));
    return window.workerStops.at(-1) - editedAt;
  });
  assert.ok(Number.isFinite(latency) && latency >= 0 && latency < 100, `worker stopped after ${latency} ms`);
  t.diagnostic(`H3 worker stopped ${latency.toFixed(1)} ms after the edit`);

  await page.locator('.answer:not(.stale)').waitFor({ timeout: 15_000 });
  assert.equal(await page.locator('.answer .value').first().innerText(), '57');
});
