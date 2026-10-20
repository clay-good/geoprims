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
  assert.match(before[0], /NM in \d+ legs/);
  await page.waitForFunction(() => document.querySelector('.workflow-inputs input') && !document.querySelector('.stale'));
  // More wind: every leg, the climb, and the fuel change; the sunset does not.
  await page.fill('#wf-wind_speed', '35 kt');
  await page.waitForFunction(() => location.hash.includes('wind_speed=35'));
  await page.waitForFunction((old) => document.querySelectorAll('.workflow-steps .sentence')[2]?.textContent !== old, before[2]);
  const after = await sentences(page);
  for (const i of [0, 1, 2]) assert.notEqual(after[i], before[i], `step ${i + 1} did not follow the wind`);
  assert.equal(after[3], before[3], 'the sunset does not depend on the wind');
  // Reopen the permalink: the edit and its answers come back.
  const link = page.url();
  const again = await browser.newPage();
  await again.goto(link);
  await again.waitForFunction(() => document.querySelector('#wf-wind_speed')?.value === '35 kt');
  await again.waitForFunction((want) => document.querySelectorAll('.workflow-steps .sentence')[2]?.textContent === want, after[2]);
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

// Equipment profiles (web/workflows, "Workflow inputs filled from profiles",
// scenario "Mapping flight from a camera and a drone").
test('a mapping flight fills from a camera and a drone profile, and the assumptions name both', { timeout: 120_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const context = await browser.newContext();
  const profile = (kind, name, tools) => ({ format: 'geoprims-profile', version: 1, kind, name, savedOn: '2026-10-19', tools });
  const store = {
    camera: { P1: profile('camera', 'P1', { 'drone.photogrammetry.gsd': { sensor_width: '35.9 mm', sensor_height: '24 mm', image_width: 8192, image_height: 5460, focal_length: '35 mm' } }) },
    drone: { M3E: profile('drone', 'M3E', { 'drone.mission.sorties': { flight_time: '38 min', reserve: 25 } }) },
  };
  await context.addInitScript((s) => localStorage.setItem('gp-profiles', s), JSON.stringify(store));
  const page = await context.newPage();
  await page.goto(`${origin}/workflows/mapping-flight/`);
  await page.waitForFunction(() => document.querySelector('astro-island:not([ssr])'));
  for (const kind of ['camera', 'drone']) {
    await page.locator('summary', { hasText: `Use my ${kind}` }).click();
    await page.getByRole('button', { name: `Fill from this ${kind}` }).click();
  }
  await page.waitForFunction(() => !document.querySelector('.workflow-answer.stale'));
  assert.equal(await page.locator('#wf-sensor_width').inputValue(), '35.9 mm');
  assert.equal(await page.locator('#wf-focal_length').inputValue(), '35 mm');
  assert.equal(await page.locator('#wf-flight_time').inputValue(), '38 min');
  assert.equal(await page.locator('#wf-reserve').inputValue(), '25');
  const named = await page.locator('.assumptions .from-profile').allTextContents();
  assert.deepEqual(named.map((s) => s.trim()).sort(), ['Camera: from profile P1', 'Drone: from profile M3E']);
  // Editing a filled value drops that profile from the assumptions.
  await page.locator('#wf-flight_time').fill('30 min');
  assert.deepEqual((await page.locator('.assumptions .from-profile').allTextContents()).map((s) => s.trim()), ['Camera: from profile P1']);
});
