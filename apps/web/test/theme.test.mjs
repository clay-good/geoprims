// Visual theme (web/visual-theme, Atlas): tokens only, two modes that each meet
// WCAG AA contrast, radii that never read as a pill, and the pre-paint mode
// choice, which is the saved one or paper.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import vm from 'node:vm';

const web = new URL('..', import.meta.url).pathname;
const css = readFileSync(join(web, 'src/styles/global.css'), 'utf8');
const MODES = ['paper', 'ink'];
const TEXT = ['text', 'muted', 'accent', 'caution', 'danger'];

/** Top-level and @media rule blocks: [selector, {--token: value}]. */
function blocks(src) {
  const out = [];
  const clean = src.replace(/\/\*[\s\S]*?\*\//g, '');
  for (const m of clean.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    const vars = Object.fromEntries([...m[2].matchAll(/(--[\w-]+):\s*([^;]+);/g)].map((v) => [v[1], v[2].trim()]));
    out.push([m[1].trim(), vars]);
  }
  return out;
}

/** The stylesheet without its @media print blocks (print has its own palette). */
function screenOnly(src) {
  let out = '';
  let i = 0;
  for (;;) {
    const at = src.indexOf('@media print', i);
    if (at < 0) return out + src.slice(i);
    out += src.slice(i, at);
    let depth = 0;
    let j = src.indexOf('{', at);
    for (; j < src.length; j++) {
      if (src[j] === '{') depth++;
      else if (src[j] === '}' && --depth === 0) break;
    }
    i = j + 1;
  }
}

/** A mode's tokens: :root defaults, then the mode's own block. */
function tokens(mode) {
  const all = blocks(screenOnly(css));
  const pick = (sel) => Object.assign({}, ...all.filter(([s]) => sel(s)).map(([, v]) => v));
  const t = { ...pick((s) => s === ':root' || s.startsWith(':root,')), ...pick((s) => s.includes(`data-theme='${mode}']`) && !s.includes('data-accent') && !s.includes('@')) };
  return t;
}

const lum = (hex) => {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255).map((c) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
};
const contrast = (a, b) => {
  const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
};

test('no color literal outside the token definitions', () => {
  const rules = css.replace(/\/\*[\s\S]*?\*\//g, '').replace(/--[\w-]+:\s*[^;]+;/g, '');
  const literal = /#[0-9a-fA-F]{3,8}\b|\b(rgb|rgba|hsl|hsla|hwb|lab|lch|oklab|oklch)\(|:\s*(white|black|red|green|blue|yellow|orange|gray|grey|silver)\b/;
  assert.doesNotMatch(rules, literal, 'global.css');
  const files = [];
  (function walk(d) {
    for (const f of readdirSync(d)) {
      const p = join(d, f);
      if (statSync(p).isDirectory()) walk(p);
      else if (/\.(svelte|astro)$/.test(f)) files.push(p);
    }
  })(join(web, 'src'));
  for (const f of files) {
    const src = readFileSync(f, 'utf8');
    for (const m of src.matchAll(/<style[^>]*>([\s\S]*?)<\/style>|style="([^"]*)"/g)) {
      assert.doesNotMatch(m[1] ?? m[2], literal, f);
    }
  }
});

test('both modes meet WCAG AA contrast for text and focus', () => {
  for (const mode of MODES) {
    const t = tokens(mode);
    for (const k of TEXT) {
      for (const bg of ['--bg', '--surface']) {
        const c = contrast(t[`--${k}`], t[bg]);
        assert.ok(c >= 4.5, `${mode} --${k} on ${bg}: ${c.toFixed(2)}:1`);
      }
    }
    const f = contrast(t['--focus'], t['--bg']);
    assert.ok(f >= 3, `${mode} focus ${f.toFixed(2)}:1`);
  }
  // The map: the result path keeps 3:1 over land and water (map-canvas "Result stands out").
  for (const mode of MODES) {
    const t = tokens(mode);
    for (const fill of ['--land', '--bg']) {
      const c = contrast(t['--accent'], t[fill]);
      assert.ok(c >= 3, `${mode} accent over ${fill}: ${c.toFixed(2)}:1`);
    }
  }
  // One signal accent per mode, used for focus too (Atlas, design W9).
  for (const mode of MODES) assert.equal(tokens(mode)['--focus'], tokens(mode)['--accent'], mode);
});

test('no control reads as a pill: every radius is 8 px or less', () => {
  // web/visual-theme "Design tokens": radii at most 8 px. The brand mark's
  // 1 px diamond and percentage radii on decorative shapes are the only
  // non-px values, and they are checked here too.
  const clean = css.replace(/\/\*[\s\S]*?\*\//g, '');
  for (const m of clean.matchAll(/border-radius:\s*([^;}]+)/g)) {
    for (const px of m[1].matchAll(/([\d.]+)px/g)) {
      assert.ok(Number(px[1]) <= 8, `border-radius ${m[1].trim()}`);
    }
    assert.doesNotMatch(m[1], /%/, `border-radius ${m[1].trim()}`);
  }
  const tokens = Object.fromEntries([...clean.matchAll(/(--radius[\w-]*):\s*([^;]+);/g)].map((t) => [t[1], t[2].trim()]));
  assert.deepEqual(tokens, { '--radius': '6px', '--radius-lg': '8px' });
});

test('the mode is chosen before first paint: the saved choice, else paper', () => {
  const html = readFileSync(join(web, 'dist/index.html'), 'utf8');
  const script = /<script>(\(\(\)=>\{const d=document\.documentElement[\s\S]*?)<\/script>/.exec(html)?.[1];
  assert.ok(script, 'inline mode script');
  assert.ok(html.indexOf(script) < html.indexOf('rel="stylesheet"'), 'runs before the stylesheet');
  const run = ({ light = false, dark = false, more = false, saved = {} }) => {
    const root = { dataset: {}, style: { setProperty: (k, v) => (root[k] = v) } };
    vm.runInNewContext(script, {
      document: { documentElement: root },
      matchMedia: (q) => ({ matches: (q.includes('contrast') && more) || (q.includes('light') && light) || (q.includes('dark') && dark) }),
      localStorage: { getItem: (k) => saved[k] ?? null },
    });
    return root;
  };
  // The OS preference no longer picks a mode: a first visit is paper either way.
  assert.equal(run({ light: true }).dataset.theme, 'paper');
  assert.equal(run({ dark: true }).dataset.theme, 'paper', 'a dark-mode machine still opens in paper');
  assert.equal(run({ more: true, dark: true }).dataset.theme, 'paper');
  assert.equal(run({}).dataset.theme, 'paper');
  // An explicit choice wins, and the retired mode names carry over to their replacement.
  assert.equal(run({ saved: { 'gp-theme': 'ink' }, light: true }).dataset.theme, 'ink');
  assert.equal(run({ saved: { 'gp-theme': 'paper' }, dark: true }).dataset.theme, 'paper');
  for (const retired of ['night', 'hud']) assert.equal(run({ saved: { 'gp-theme': retired } }).dataset.theme, 'ink', retired);
  for (const retired of ['sunlight', 'high-contrast', 'daylight']) {
    assert.equal(run({ saved: { 'gp-theme': retired } }).dataset.theme, 'paper', retired);
  }
  assert.equal(run({ saved: { 'gp-theme': 'bogus' }, dark: true }).dataset.theme, 'paper');
});

test('fonts are self-hosted Geist, subset within the profile\'s budget', () => {
  const budget = JSON.parse(readFileSync(join(web, '../../data/reference-profile.json'), 'utf8')).budgets.fontBytes.hard;
  const dir = join(web, 'dist/fonts');
  const files = readdirSync(dir).filter((f) => f.endsWith('.woff2'));
  assert.deepEqual(files.sort(), ['Geist-Variable-subset.woff2', 'GeistMono-Variable-subset.woff2']);
  const bytes = files.reduce((n, f) => n + statSync(join(dir, f)).size, 0);
  assert.ok(bytes <= budget, `${bytes} bytes of fonts, budget ${budget}`);
  assert.match(readFileSync(join(dir, 'OFL.txt'), 'utf8'), /SIL Open Font License/);
  assert.match(css, /font-family: 'Geist';[\s\S]*?font-display: swap/);
});

test('the header toggle switches modes, saves the choice, and relabels itself', async () => {
  // web/visual-theme "Theme modes": one control, stating the mode it switches to.
  const el = (cls) => ({ className: cls, textContent: '', innerHTML: '' });
  const label = el('label');
  const svg = el('svg');
  const button = {
    attrs: {},
    title: '',
    listeners: {},
    querySelector: (sel) => (sel === '.label' ? label : svg),
    setAttribute(k, v) {
      this.attrs[k] = v;
    },
    addEventListener(name, fn) {
      this.listeners[name] = fn;
    },
    click() {
      this.listeners.click();
    },
  };
  const root = { dataset: { theme: 'paper' } };
  const store = {};
  globalThis.document = { documentElement: root };
  globalThis.localStorage = { setItem: (k, v) => (store[k] = v) };
  const { wireTheme } = await import('../src/lib/display.js');
  wireTheme(button);

  // In paper, it offers dark.
  assert.equal(label.textContent, 'Dark');
  assert.equal(button.attrs['aria-label'], 'Switch to dark mode');
  assert.match(svg.innerHTML, /M21 12\.79/, 'the moon');

  button.click();
  assert.equal(root.dataset.theme, 'ink');
  assert.equal(store['gp-theme'], 'ink');
  assert.equal(label.textContent, 'Light');
  assert.equal(button.attrs['aria-label'], 'Switch to light mode');
  assert.match(svg.innerHTML, /<circle cx="12"/, 'the sun');

  button.click();
  assert.equal(root.dataset.theme, 'paper');
  assert.equal(store['gp-theme'], 'paper');
  assert.equal(label.textContent, 'Dark');
  delete globalThis.document;
  delete globalThis.localStorage;
});

// Night use (ux/mobile-and-field, "Sunlight and night use"). The separate
// `night` and `sunlight` modes were folded into ink and paper by
// redesign-minimal-shell, which allows the header exactly one display
// control. What that requirement asks of night is checked here against ink.
test('ink shows no bright surface, so nothing flares in a dark cockpit', () => {
  const t = tokens('ink');
  // Every surface a page or a map paints at full size, in relative luminance.
  for (const k of ['--bg', '--surface', '--line', '--land', '--graticule', '--backdrop']) {
    const l = lum(t[k]);
    assert.ok(l <= 0.05, `ink ${k} has luminance ${l.toFixed(3)}`);
  }
  // The canvas is the page, not a white sheet inside it.
  assert.ok(lum(t['--land']) <= lum(t['--surface']) + 0.01, 'ink map land is brighter than the page');
});

test('ink never paints a bright frame while a page loads', () => {
  const html = readFileSync(join(web, 'dist/index.html'), 'utf8');
  // The mode is on the document before the stylesheet, and the body's
  // background is a token, so the first paint is already in the right mode.
  const body = /(?:^|\})\s*body\s*\{([^}]*)\}/.exec(css.replace(/\/\*[\s\S]*?\*\//g, ''))?.[1] ?? '';
  assert.match(body, /background:\s*var\(--bg\)/);
  assert.ok(html.includes("d.dataset.theme=t==='ink'?'ink':'paper'"), 'the mode is not set before first paint');
});

test('a state is readable without seeing its color', () => {
  // Red light distorts color, so every status phrase carries a mark and words.
  const app = readFileSync(join(web, 'src/components/ToolApp.svelte'), 'utf8');
  const marks = /const STATUS_MARK = \{([^}]*)\}/.exec(app)?.[1] ?? '';
  for (const word of ['Within', 'Near', 'Beyond', 'Meets']) {
    assert.match(marks, new RegExp(`${word}:`), `no mark for a ${word} status`);
  }
  // The phrase itself is words, and the mark is decoration beside it.
  assert.match(app, /class="mark" aria-hidden="true">\{st\.mark\}<\/span> <strong>\{st\.phrase\}/);
});
