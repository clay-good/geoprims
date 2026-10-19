// Equipment profiles (web/equipment-profiles, "Profile manager" scenario
// "Save from a tool", "Filling a tool", and "Permalink carries values, not
// the name"): a camera saved on the GSD tool fills the trigger tool with the
// same values, the answer card names the profile until a kept field changes,
// and the name never reaches the link.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { serveBuiltSite } from './site.mjs';

const CAMERA = { sensor_width: '35.9 mm', sensor_height: '24 mm', image_width: '8192', image_height: '5460', focal_length: '35 mm' };
const NAME = 'Zenmuse sentinel P1';

const settle = (page) => page.waitForFunction(() => document.querySelector('.card.answer .sentence') && !document.querySelector('.card.answer.stale'));
const openMore = async (page) => {
  for (const d of await page.locator('details:has(> summary:text-matches("More options"))').all()) {
    if (!(await d.evaluate((el) => el.open))) await d.locator('> summary').click();
  }
};

test('a camera saved on one tool fills another, and the chip follows edits', { timeout: 120_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const page = await browser.newPage();

  await page.goto(`${origin}/drone/photogrammetry/gsd/`);
  await settle(page);
  await openMore(page);
  for (const [k, v] of Object.entries(CAMERA)) await page.locator(`#field-${k}`).fill(v);
  await settle(page);
  await page.locator('summary', { hasText: 'Camera profiles' }).click();
  await page.getByLabel('Profile name').fill(NAME);
  await page.getByRole('button', { name: "Save this tool's camera data" }).click();
  await page.getByText(`Saved this tool's camera data to ${NAME}.`).waitFor();

  await page.goto(`${origin}/drone/photogrammetry/trigger/`);
  await settle(page);
  await page.locator('summary', { hasText: 'Camera profiles' }).click();
  await page.getByRole('button', { name: 'Load', exact: true }).click();
  await settle(page);
  await openMore(page);
  for (const [k, v] of Object.entries(CAMERA)) assert.equal(await page.locator(`#field-${k}`).inputValue(), v, k);
  assert.equal((await page.locator('.profile-chip').textContent()).trim(), `From profile: ${NAME}`);
  assert.equal(new URL(page.url()).hash.includes('sentinel'), false, 'the profile name reached the permalink');

  await page.locator('#field-focal_length').fill('24 mm');
  await settle(page);
  assert.equal((await page.locator('.profile-chip').textContent()).trim(), `Edited from ${NAME}`);
});
