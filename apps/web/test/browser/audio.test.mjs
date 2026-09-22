// Interface sounds rendered for real (web/audio-feedback; 6.1): every sound
// through an OfflineAudioContext, measured for length, peak level, and pitch;
// mute silencing within 50 ms; and no audio request when sound is on.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { serveBuiltSite, web } from './site.mjs';

const module = readFileSync(join(web, 'src/lib/audio.js'), 'utf8').replace(/^export /gm, '');

test('every sound, rendered offline: at most 120 ms, at most -12 dBFS, error unlike success', { timeout: 120_000 }, async (t) => {
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const page = await browser.newPage();
  const out = await page.evaluate(async (src) => {
    (0, eval)(`${src}; window.__audio = { SOUNDS, voice, PEAK };`);
    const A = window.__audio;
    const rate = 48000;
    const measure = async (name, muteAt) => {
      const ctx = new OfflineAudioContext(1, rate * 0.3, rate);
      const master = ctx.createGain();
      master.connect(ctx.destination);
      if (muteAt !== undefined) master.gain.setValueAtTime(0, muteAt);
      A.voice(ctx, master, name, 0);
      const d = (await ctx.startRendering()).getChannelData(0);
      let peak = 0, last = 0, crossings = 0;
      for (let i = 0; i < d.length; i++) {
        const a = Math.abs(d[i]);
        if (a > peak) peak = a;
        if (a > 1e-4) last = i;
        if (i && d[i - 1] < 0 && d[i] >= 0) crossings++;
      }
      return { ms: (last / rate) * 1000, dbfs: 20 * Math.log10(peak || 1e-12), hz: crossings / ((last || 1) / rate) };
    };
    const all = {};
    for (const name of Object.keys(A.SOUNDS)) all[name] = await measure(name);
    all.mutedAt30 = await measure('error', 0.03);
    return all;
  }, module);
  for (const [name, m] of Object.entries(out)) {
    if (name === 'mutedAt30') continue;
    assert.ok(m.ms <= 120, `${name}: ${m.ms.toFixed(1)} ms`);
    assert.ok(m.dbfs <= -12 + 1e-6, `${name}: ${m.dbfs.toFixed(2)} dBFS`);
  }
  assert.ok(out.error.hz < out.computed.hz / 2, `error ${out.error.hz.toFixed(0)} Hz vs computed ${out.computed.hz.toFixed(0)} Hz`);
  assert.ok(out.mutedAt30.ms <= 30 + 50, `muted at 30 ms, silent by ${out.mutedAt30.ms.toFixed(1)} ms`);
  t.diagnostic(Object.entries(out).map(([k, m]) => `${k} ${m.ms.toFixed(0)} ms ${m.dbfs.toFixed(1)} dBFS`).join(' · '));
});

test('with sound on, results play and nothing is downloaded for them', { timeout: 120_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true, args: ['--autoplay-policy=no-user-gesture-required'] });
  t.after(() => browser.close());
  const context = await browser.newContext({ serviceWorkers: 'block' });
  await context.addInitScript(() => localStorage.setItem('gp-audio', 'true'));
  const page = await context.newPage();
  const audioRequests = [];
  page.on('request', (r) => /\.(mp3|wav|ogg|m4a|aac|flac|opus)(\?|$)/i.test(r.url()) && audioRequests.push(r.url()));
  await page.goto(`${origin}/aviation/altimetry/density-altitude/`);
  await page.waitForSelector('.card.answer');
  assert.equal(await page.isVisible('[data-audio-toggle]'), true, 'the header mute control shows once audio is on');
  await page.click('#field-elevation');
  await page.keyboard.type('0');
  await page.waitForTimeout(800);
  // m mutes, and says so.
  await page.locator('body').click({ position: { x: 5, y: 5 } });
  await page.keyboard.press('m');
  await page.waitForTimeout(300);
  assert.match(await page.$eval('.toast', (e) => e.textContent), /Audio muted/);
  assert.equal(await page.getAttribute('[data-audio-toggle]', 'aria-pressed'), 'true');
  assert.deepEqual(audioRequests, []);
});

test('first visit: no sound control in the header, and no audio module loaded', { timeout: 120_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const page = await (await browser.newContext({ serviceWorkers: 'block' })).newPage();
  const scripts = [];
  page.on('request', (r) => r.resourceType() === 'script' && scripts.push(r.url()));
  await page.goto(`${origin}/aviation/altimetry/density-altitude/`);
  await page.waitForSelector('.card.answer');
  await page.click('#field-elevation');
  await page.keyboard.type('0');
  await page.waitForTimeout(800);
  assert.equal(await page.isVisible('[data-audio-toggle]'), false);
  assert.equal(await page.evaluate(() => typeof AudioContext !== 'undefined' && performance.getEntriesByType('resource').some((e) => /audio\.[\w-]+\.js/.test(e.name))), false, 'audio.js is not loaded');
});
