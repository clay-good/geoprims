// Workflow page, live (add-job-workflows task 4): editing one input reruns
// every step that depends on it, the permalink restores the edit, and a step
// that fails stops the chain with its own error while later steps wait.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { serveBuiltSite } from './site.mjs';

const sentences = (page) => page.$$eval('.workflow-steps .journey-step', (els) => els.map((e) => e.querySelector('.sentence, .help')?.textContent ?? ''));

test('a changed input reruns the steps that use it, and the permalink restores it', { timeout: 120_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const page = await browser.newPage();
  await page.goto(`${origin}/workflows/vfr-cross-country/`);
  const before = await sentences(page);
  assert.equal(before.length, 4);
  // The answers are in the HTML before any script runs.
  assert.match(before[1], /heading/i);
  await page.waitForFunction(() => document.querySelector('.workflow-inputs input') && !document.querySelector('.stale'));
  await page.fill('#wf-tas', '140 kt');
  await page.waitForFunction(() => location.hash.includes('tas=140'));
  await page.waitForFunction((old) => document.querySelectorAll('.workflow-steps .sentence')[1]?.textContent !== old, before[1]);
  const after = await sentences(page);
  assert.equal(after[0], before[0], 'the distance does not depend on the airspeed');
  for (const i of [1, 2, 3]) assert.notEqual(after[i], before[i], `step ${i + 1} did not follow the airspeed`);
  // Reopen the permalink: the edit and its answers come back.
  const link = page.url();
  const again = await browser.newPage();
  await again.goto(link);
  await again.waitForFunction(() => document.querySelector('#wf-tas')?.value === '140 kt');
  await again.waitForFunction((want) => document.querySelectorAll('.workflow-steps .sentence')[1]?.textContent === want, after[1]);
});

test('a step that fails shows its own error, and later steps wait', { timeout: 120_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const page = await browser.newPage();
  await page.goto(`${origin}/workflows/preflight-check/`);
  await page.waitForFunction(() => document.querySelector('#wf-metar'));
  await page.waitForTimeout(500);
  await page.fill('#wf-metar', 'not a metar');
  await page.waitForSelector('.journey-step.failed');
  const waiting = await page.$$eval('.journey-step.waiting .help', (els) => els.map((e) => e.textContent));
  assert.equal(waiting.filter((x) => x === 'Waiting for step 1.').length, 3);
  assert.match(await page.textContent('.workflow-answer .sentence'), /^Stopped at step 1 \(METAR decoder\):/);
});
