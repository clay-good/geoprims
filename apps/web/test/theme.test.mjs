// Visual theme (web/visual-theme): tokens only, five modes that each meet
// WCAG AA contrast, night mode's luminance limits, and the pre-paint mode
// choice that follows the OS preference.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import vm from 'node:vm';

const web = new URL('..', import.meta.url).pathname;
const css = readFileSync(join(web, 'src/styles/global.css'), 'utf8');
const MODES = ['hud', 'daylight', 'sunlight', 'night', 'high-contrast'];
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

/** A mode's tokens: :root defaults, then the mode's own block (night at full brightness). */
function tokens(mode) {
  const all = blocks(css);
  const pick = (sel) => Object.assign({}, ...all.filter(([s]) => sel(s)).map(([, v]) => v));
  const t = { ...pick((s) => s === ':root' || s.startsWith(':root,')), ...pick((s) => s.includes(`data-theme='${mode}']`) && !s.includes('data-accent') && !s.includes('@')) };
  for (const [k, v] of Object.entries(t)) {
    const hex = /#[0-9a-f]{6}/i.exec(v);
    if (hex) t[k] = hex[0]; // color-mix at --dim = 1 is its first color
  }
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

test('every mode meets WCAG AA contrast for text and focus', () => {
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
  const green = { ...tokens('hud'), '--accent': blocks(css).find(([s]) => s.includes("data-accent='green'"))[1]['--accent'] };
  assert.ok(contrast(green['--accent'], green['--surface']) >= 4.5, 'hud green accent');
});

test('night mode stays dark-adapted: text luminance 0.175-0.25, backgrounds at most 0.005', () => {
  const t = tokens('night');
  for (const k of [...TEXT, 'focus']) {
    const l = lum(t[`--${k}`]);
    assert.ok(l >= 0.175 && l <= 0.25, `--${k} luminance ${l.toFixed(3)}`);
  }
  for (const k of ['--bg', '--surface']) assert.ok(lum(t[k]) <= 0.005, `${k} luminance ${lum(t[k]).toFixed(4)}`);
});

test('the mode is chosen before first paint from the saved choice or the OS preference', () => {
  const html = readFileSync(join(web, 'dist/index.html'), 'utf8');
  const script = /<script>(\(\(\)=>\{const d=document\.documentElement;[\s\S]*?)<\/script>/.exec(html)?.[1];
  assert.ok(script, 'inline mode script');
  assert.ok(html.indexOf(script) < html.indexOf('rel="stylesheet"'), 'runs before the stylesheet');
  const run = ({ light = false, more = false, saved = {} }) => {
    const root = { dataset: {}, style: { setProperty: (k, v) => (root[k] = v) } };
    vm.runInNewContext(script, {
      document: { documentElement: root },
      matchMedia: (q) => ({ matches: (q.includes('contrast') && more) || (q.includes('light') && light) }),
      localStorage: { getItem: (k) => saved[k] ?? null },
    });
    return root;
  };
  assert.equal(run({}).dataset.theme, 'hud');
  assert.equal(run({ light: true }).dataset.theme, 'daylight');
  assert.equal(run({ more: true, light: true }).dataset.theme, 'high-contrast');
  const night = run({ saved: { 'gp-theme': 'night', 'gp-dim': '0.5' } });
  assert.equal(night.dataset.theme, 'night');
  assert.equal(night['--dim'], '0.5');
  assert.equal(run({ saved: { 'gp-accent': 'green' } }).dataset.accent, 'green');
});
