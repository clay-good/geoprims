// Diagram labels, measured in a real browser (align-visuals-with-jobs task 8):
// every diagram, drawn from its worked example at phone width, keeps each
// label inside its 320 × 240 drawing and clear of every other label.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { nodeHost } from '../../../../packages/runtime/src/node.mjs';
import { DIAGRAM_TOOLS, diagram } from '../../src/lib/diagrams.js';
import { web } from './site.mjs';

const root = join(web, '../..');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const host = nodeHost(join(root, 'dist/wasm'));
const css = readFileSync(join(web, 'src/styles/global.css'), 'utf8');

/** How far two label boxes may touch (px): antialiasing and the halo, not a collision. */
const SLACK = 1.5;

test('every diagram’s labels stay inside the drawing and clear of each other', { timeout: 120_000 }, async (t) => {
  const drawings = [];
  for (const id of DIAGRAM_TOOLS) {
    const tool = catalog.tools.find((x) => x.id === id);
    const ex = (tool.examples.find((e) => e.id === tool['x-primary-example']) ?? tool.examples[0]).input;
    const d = diagram(id, ex, JSON.parse(await host.invoke(id, JSON.stringify(ex))));
    assert.ok(d, `${id}: no diagram`);
    drawings.push({ id, markup: d.markup });
  }
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  // A phone: the drawing at 390 px wide, scaled from its 320-unit viewBox.
  const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  await page.setContent(`<!doctype html><style>${css}</style><body>${drawings.map((d) => `<div data-id="${d.id}" style="width:358px">${d.markup}</div>`).join('')}</body>`);
  const problems = await page.evaluate((slack) => {
    const out = [];
    for (const box of document.querySelectorAll('[data-id]')) {
      const svg = box.querySelector('svg');
      const s = svg.getBoundingClientRect();
      const k = s.width / 320;
      const labels = [...svg.querySelectorAll('text')].filter((el) => el.textContent.trim()).map((el) => {
        const r = el.getBoundingClientRect();
        return { text: el.textContent.trim(), x0: (r.left - s.left) / k, y0: (r.top - s.top) / k, x1: (r.right - s.left) / k, y1: (r.bottom - s.top) / k };
      });
      for (const l of labels) {
        if (l.x0 < -slack || l.y0 < -slack || l.x1 > 320 + slack || l.y1 > 240 + slack) out.push(`${box.dataset.id}: "${l.text}" leaves the drawing`);
      }
      for (let i = 0; i < labels.length; i++) {
        for (let j = i + 1; j < labels.length; j++) {
          const [a, b] = [labels[i], labels[j]];
          const w = Math.min(a.x1, b.x1) - Math.max(a.x0, b.x0);
          const h = Math.min(a.y1, b.y1) - Math.max(a.y0, b.y0);
          if (w > slack && h > slack) out.push(`${box.dataset.id}: "${a.text}" overlaps "${b.text}"`);
        }
      }
    }
    return out;
  }, SLACK);
  assert.deepEqual(problems, []);
  assert.ok(drawings.length >= 40, `${drawings.length} diagrams measured`);
});
