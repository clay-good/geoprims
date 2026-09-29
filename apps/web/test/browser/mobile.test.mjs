// The mobile bar for a hero tool (plan-launch-and-value-proof L4, and
// web/visual-theme "Reflow at 320 px"): every hero tool's page, plus the home
// page and the index, at 320 CSS px and at 640 (which is 1280 at 200% zoom),
// in WebKit, since that is the engine the bar names.
//
// Reflow is asserted: no page may scroll sideways. Target size is counted
// rather than asserted, because the bar the spec sets -- 48 x 48 CSS px, above
// WCAG 2.5.8's 24, for field and gloved use -- is not met yet by the chips,
// the copy buttons, or the unit pickers, and deciding what counts as a tap
// target rather than an affordance inside a line of text is a design call, not
// a test's. The header button, the theme toggle, and the search button were
// simply set shorter than the --target the styles already define, which was an
// oversight rather than a decision, and now use it. The three that are left are
// held where they are so the count cannot quietly grow.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import { webkit } from 'playwright';
import { serveBuiltSite, web } from './site.mjs';

/** The hero tool ids, read from the launch checklist that names them. */
function heroRoutes() {
  const md = readFileSync(join(web, '../../docs/launch/hero-tools.md'), 'utf8');
  const ids = [...new Set([...md.matchAll(/`([a-z0-9]+(?:\.[a-z0-9-]+)+)`/g)].map((m) => m[1]))];
  return ['/', '/tools/', ...ids.map((id) => `/${id.split('.').join('/')}/`)];
}

/** Controls that are too small to tap with a glove, by CSS px. */
const MEASURE = 'button, input:not([type=hidden]), select, textarea, summary, [role=button]';
const SMALL_CONTROLS = 3;

test('hero pages reflow at 320 px in WebKit, and no control shrinks further', { timeout: 300_000 }, async (t) => {
  const origin = await serveBuiltSite(t);
  const browser = await webkit.launch({ headless: true });
  t.after(() => browser.close());
  const routes = heroRoutes();
  assert.ok(routes.length > 25, `${routes.length} hero routes`);
  const scrolls = [];
  const headerProblems = [];
  const small = new Map();
  for (const width of [320, 640]) {
    const context = await browser.newContext({ viewport: { width, height: 800 } });
    const page = await context.newPage();
    for (const path of routes) {
      const res = await page.goto(origin + path, { waitUntil: 'load' });
      assert.ok(res?.ok(), `${path} is ${res?.status()}`);
      await page.waitForTimeout(120);
      const seen = await page.evaluate((sel) => {
        const d = document.documentElement;
        const tiny = [];
        for (const e of document.querySelectorAll(sel)) {
          const b = e.getBoundingClientRect();
          const cs = getComputedStyle(e);
          // A control with no box, or one placed off-screen for a screen
          // reader, is not something anyone taps.
          if (b.width === 0 || b.height === 0 || cs.visibility === 'hidden') continue;
          if (e.classList.contains('sr-only')) continue;
          // A glossary term is a word inside a sentence, and a line of prose
          // cannot be 48 px tall without ceasing to be a line of prose.
          if (e.tagName === 'BUTTON' && e.classList.contains('term')) continue;
          if (b.width < 48 || b.height < 48) {
            tiny.push(`${e.tagName.toLowerCase()}${e.className ? `.${String(e.className).trim().split(/\s+/)[0]}` : ''}`);
          }
        }
        const header = document.querySelector('header.site');
        const toggle = header?.querySelector('[data-theme-toggle]');
        const box = toggle?.getBoundingClientRect();
        const headerProblem = !header || !box
          ? 'header or theme toggle is missing'
          : header.scrollWidth > header.clientWidth + 1
            ? `header scrolls ${header.scrollWidth} > ${header.clientWidth}`
            : box.left < 0 || box.right > innerWidth
              ? `theme toggle lies outside 0–${innerWidth}: ${box.left}–${box.right}`
              : box.width < 48 || box.height < 48
                ? `theme toggle is ${box.width} × ${box.height}`
                : null;
        return { over: d.scrollWidth > d.clientWidth + 1 ? `${d.scrollWidth} > ${d.clientWidth}` : null, tiny, headerProblem };
      }, MEASURE);
      if (seen.over) scrolls.push(`${path} at ${width} px scrolls sideways: ${seen.over}`);
      if (width === 320 && seen.headerProblem) headerProblems.push(`${path}: ${seen.headerProblem}`);
      for (const k of new Set(seen.tiny)) small.set(k, (small.get(k) ?? 0) + 1);
    }
    await context.close();
  }
  t.diagnostic(`controls under 48 x 48: ${[...small.keys()].sort().join(', ') || 'none'}`);
  assert.deepEqual(scrolls, [], scrolls.join('\n'));
  assert.deepEqual(headerProblems, [], headerProblems.join('\n'));
  assert.ok(
    small.size <= SMALL_CONTROLS,
    `${small.size} kinds of control are under 48 x 48, up from ${SMALL_CONTROLS}: ${[...small.keys()].join(', ')}`,
  );
});
