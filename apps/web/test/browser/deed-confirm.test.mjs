import assert from 'node:assert/strict';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { serveBuiltSite } from './site.mjs';

test('a deed call can be reviewed, edited, and confirmed into the plotter', { timeout: 120_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const page = await browser.newPage();
  await page.goto(`${origin}/survey/land/deed-parse/`);
  await page.waitForSelector('.deed-confirm');

  const marks = page.locator('.deed-source mark');
  assert.equal(await marks.count(), 4);
  await marks.nth(1).click();
  assert.equal(await page.locator('.deed-calls tbody tr.selected th').textContent(), '2');

  await page.getByLabel('Call 1 Direction').fill('S 0 E');
  await page.getByRole('button', { name: 'Confirm calls and open deed plot' }).click();
  await page.waitForURL(/\/survey\/land\/deed-plot\/#v1:/);
  await page.waitForFunction(() => document.querySelector('.card.answer .sentence') && !document.querySelector('.card.answer.stale'));
  assert.match(await page.locator('#field-calls').inputValue(), /S 0 E/);
  assert.match(await page.locator('.card.answer .sentence').textContent(), /The deed (closes|leaves)/);
});
