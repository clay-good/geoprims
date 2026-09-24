#!/usr/bin/env node
// Post-build Open Graph cards (discovery/search-pages, "Open Graph images").
// Every page whose head names an og:image gets a 1200 × 630 JPEG: the page's
// name, where it sits, and for a tool the worked example's answer, drawn over
// a contour sheet seeded by the page's path so each card has its own terrain.
// Nothing renders at runtime. Base.astro carries the card's words on the
// og:image tag as data-og-* attributes; this script reads them, renders, and
// strips them, so the shipped HTML keeps only the standard tags.
//
// Rendering goes through Playwright's Chromium with the site's own fonts
// embedded. Cards are cached by a hash of their HTML, so a rebuild renders
// only the cards whose words changed.
import { createHash } from 'node:crypto';
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { contourSegments, heightField } from '../src/lib/terrain.js';

const web = new URL('..', import.meta.url).pathname;
const dist = join(web, 'dist');
const cache = join(web, 'node_modules/.cache/og');
export const WIDTH = 1200;
export const HEIGHT = 630;
const OG_TAG = /<meta property="og:image" content="([^"]+)"([^>]*?)\s*\/?>/;

const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);
const unesc = (s) => String(s).replace(/&(amp|lt|gt|quot|#39|#x27);/g, (_, e) => ({ amp: '&', lt: '<', gt: '>', quot: '"', '#39': "'", '#x27': "'" })[e]);
const attr = (tag, name) => unesc(new RegExp(`${name}="([^"]*)"`).exec(tag)?.[1] ?? '');
const meta = (html, prop) => unesc(new RegExp(`<meta property="${prop}" content="([^"]*)"`).exec(html)?.[1] ?? '');

/** A path's own terrain: contour lines over the home page's height field, offset by a hash of the path. */
export function contours(key) {
  const h = createHash('sha256').update(key).digest();
  const field = heightField(1609);
  const ox = h.readUInt16BE(0) / 97;
  const oy = h.readUInt16BE(2) / 97;
  const cell = 20;
  const cols = WIDTH / cell + 1;
  const rows = HEIGHT / cell + 1;
  const grid = new Float32Array(cols * rows);
  for (let j = 0; j < rows; j += 1) for (let i = 0; i < cols; i += 1) grid[j * cols + i] = field(ox + i * 0.045, oy + j * 0.045);
  const lines = [];
  for (let k = 1; k < 20; k += 1) {
    const d = contourSegments(grid, cols, rows, k / 20, cell).map(([[x1, y1], [x2, y2]]) => `M${x1.toFixed(1)} ${y1.toFixed(1)}L${x2.toFixed(1)} ${y2.toFixed(1)}`).join('');
    if (d) lines.push(`<path class="${k % 5 === 0 ? 'ix' : 'c'}" d="${d}"/>`);
  }
  return lines.join('');
}

/** The answer's size: a short number reads large, a long reading steps down. */
const answerSize = (s) => (s.length <= 10 ? 132 : s.length <= 16 ? 96 : 68);

/** The card's HTML: self-contained, fonts inline, nothing fetched. */
export function cardHtml({ key, eyebrow, title, answer, caption }, fonts) {
  const titleSize = title.length <= 22 ? 84 : title.length <= 40 ? 68 : 56;
  return `<!doctype html><html><head><meta charset="utf-8"><style>
@font-face{font-family:Geist;src:url(data:font/woff2;base64,${fonts.sans}) format('woff2');font-weight:100 900}
@font-face{font-family:'Geist Mono';src:url(data:font/woff2;base64,${fonts.mono}) format('woff2');font-weight:100 900}
*{box-sizing:border-box;margin:0}
body{width:${WIDTH}px;height:${HEIGHT}px;overflow:hidden;background:#f3f4f1;color:#0c1116;font-family:Geist,sans-serif;position:relative}
svg.t{position:absolute;inset:0;-webkit-mask-image:linear-gradient(90deg,transparent 18%,#000 62%)}
svg.t path{fill:none;stroke-linecap:round}
svg.t .c{stroke:#c6cfc3;stroke-width:1.4}
svg.t .ix{stroke:#93a192;stroke-width:2}
.reticle{position:absolute;right:150px;top:250px;width:120px;height:120px;color:#ff5a1f}
.frame{position:absolute;inset:0;padding:64px 72px 56px;display:flex;flex-direction:column}
.brand{display:flex;align-items:center;gap:14px;font-family:'Geist Mono';font-weight:700;font-size:30px;letter-spacing:-.01em}
.brand svg{color:#ff5a1f}
.eyebrow{margin-top:52px;font-family:'Geist Mono';font-size:24px;letter-spacing:.14em;text-transform:uppercase;color:#b53c0a}
h1{margin-top:14px;font-size:${titleSize}px;line-height:1.02;letter-spacing:-.035em;font-weight:700;max-width:960px;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden}
.answer{margin-top:22px;font-size:${answerSize(answer)}px;line-height:1;font-weight:700;letter-spacing:-.03em;color:#b53c0a;font-variant-numeric:tabular-nums;max-width:900px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.caption{margin-top:20px;font-size:28px;line-height:1.35;color:#4f5763;max-width:820px;display:-webkit-box;-webkit-line-clamp:${answer ? 2 : 3};-webkit-box-orient:vertical;overflow:hidden}
.foot{margin-top:auto;display:flex;justify-content:space-between;align-items:center;font-family:'Geist Mono';font-size:22px;color:#4f5763;border-top:2px solid #d9ddd6;padding-top:22px}
.foot b{color:#0c1116;font-weight:600}
</style></head><body>
<svg class="t" viewBox="0 0 ${WIDTH} ${HEIGHT}" width="${WIDTH}" height="${HEIGHT}">${contours(key)}</svg>
<svg class="reticle" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"><circle cx="12" cy="12" r="7.5"/><path d="M12 1.5v5M12 17.5v5M1.5 12h5M17.5 12h5"/><circle cx="12" cy="12" r="1.3" fill="currentColor" stroke="none"/></svg>
<div class="frame">
<div class="brand"><svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><circle cx="12" cy="12" r="7.5"/><path d="M12 1.5v5M12 17.5v5M1.5 12h5M17.5 12h5"/><circle cx="12" cy="12" r="1.3" fill="currentColor" stroke="none"/></svg>geoprims</div>
${eyebrow ? `<p class="eyebrow">${esc(eyebrow)}</p>` : '<p class="eyebrow">&nbsp;</p>'}
<h1>${esc(title)}</h1>
${answer ? `<p class="answer">${esc(answer)}</p>` : ''}
${caption ? `<p class="caption">${esc(caption)}</p>` : ''}
<p class="foot"><b>geoprims.com</b><span>Free · cited · runs on your device</span></p>
</div></body></html>`;
}

function pages(dir, out = []) {
  for (const f of readdirSync(dir)) {
    const p = join(dir, f);
    if (statSync(p).isDirectory()) pages(p, out);
    else if (f.endsWith('.html')) out.push(p);
  }
  return out;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const fonts = {
    sans: readFileSync(join(web, 'public/fonts/Geist-Variable-subset.woff2')).toString('base64'),
    mono: readFileSync(join(web, 'public/fonts/GeistMono-Variable-subset.woff2')).toString('base64'),
  };
  const jobs = new Map();
  for (const file of pages(dist)) {
    const html = readFileSync(file, 'utf8');
    const m = OG_TAG.exec(html);
    if (!m) continue;
    const url = new URL(m[1]);
    // Keep only the standard tag in what ships.
    writeFileSync(file, html.replace(m[0], `<meta property="og:image" content="${m[1]}">`));
    // A preset page shares its canonical page's card, which that page draws.
    const own = `/${relative(dist, file)}`.replace(/index\.html$/, '');
    const canonical = /<link rel="canonical" href="([^"]+)"/.exec(html)?.[1];
    if (canonical && new URL(canonical).pathname !== own) continue;
    const card = {
      key: url.pathname,
      eyebrow: attr(m[2], 'data-og-eyebrow'),
      title: attr(m[2], 'data-og-title') || meta(html, 'og:title'),
      answer: attr(m[2], 'data-og-answer'),
      caption: attr(m[2], 'data-og-caption') || meta(html, 'og:description'),
    };
    if (!url.pathname.startsWith('/og/') || !url.pathname.endsWith('.jpg')) throw new Error(`${file}: og:image ${m[1]} is not under /og/`);
    if (jobs.has(url.pathname)) throw new Error(`${file}: two pages want the card at ${url.pathname}`);
    jobs.set(url.pathname, { card });
  }
  mkdirSync(cache, { recursive: true });
  let browser = null;
  let page = null;
  let rendered = 0;
  for (const [path, { card }] of jobs) {
    const html = cardHtml(card, fonts);
    const cached = join(cache, `${createHash('sha256').update(html).digest('hex').slice(0, 32)}.jpg`);
    if (!existsSync(cached)) {
      if (!page) {
        const { chromium } = await import('playwright');
        browser = await chromium.launch();
        page = await browser.newPage({ viewport: { width: WIDTH, height: HEIGHT }, deviceScaleFactor: 1 });
      }
      await page.setContent(html, { waitUntil: 'load' });
      await page.evaluate(() => document.fonts.ready);
      writeFileSync(cached, await page.screenshot({ type: 'jpeg', quality: 85 }));
      rendered += 1;
    }
    const out = join(dist, path);
    mkdirSync(dirname(out), { recursive: true });
    copyFileSync(cached, out);
  }
  await browser?.close();
  // What each card says, for the example-parity gate (test/parity.test.mjs):
  // the attributes it was drawn from are gone from the shipped pages.
  writeFileSync(join(cache, 'drawn.json'), JSON.stringify(Object.fromEntries([...jobs].map(([path, { card }]) => [path, card.answer ?? null]))));
  console.log(`og: ${jobs.size} cards (${rendered} rendered, ${jobs.size - rendered} from cache)`);
}
