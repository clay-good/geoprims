// Color-vision deficiency (web/visual-theme, "WCAG 2.2 AA conformance" and the
// 2.5 audit): each mode's colors seen through full protanopia, deuteranopia,
// and tritanopia (Machado, Oliveira, and Fernandes 2009, severity 1.0, applied
// in linear RGB). Text must stay at 4.5:1 and the result mark at 3:1 over land
// and water for every one of them, so nothing depends on seeing hue.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const web = new URL('..', import.meta.url).pathname;
const css = screenOnly(readFileSync(join(web, 'src/styles/global.css'), 'utf8').replace(/\/\*[\s\S]*?\*\//g, ''));

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

const MACHADO = {
  protanopia: [[0.152286, 1.052583, -0.204868], [0.114503, 0.786281, 0.099216], [-0.003882, -0.048116, 1.051998]],
  deuteranopia: [[0.367322, 0.860646, -0.227968], [0.280085, 0.672501, 0.047413], [-0.01182, 0.04294, 0.968881]],
  tritanopia: [[1.255528, -0.076749, -0.178779], [-0.078411, 0.930809, 0.147602], [0.004733, 0.691367, 0.3039]],
};

const toLinear = (hex) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255).map((c) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
const simulate = (lin, m) => m.map((row) => Math.min(1, Math.max(0, row[0] * lin[0] + row[1] * lin[1] + row[2] * lin[2])));
const lum = ([r, g, b]) => 0.2126 * r + 0.7152 * g + 0.0722 * b;
const ratio = (a, b) => {
  const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
};

/** A mode's color tokens as hex, from :root and the mode's own block. */
function tokens(mode) {
  const out = {};
  for (const m of css.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    const sel = m[1].trim();
    const mine = sel === ':root' || sel.startsWith(':root,') ? 0 : sel.includes(`data-theme='${mode}']`) && !sel.includes('@') ? 1 : -1;
    if (mine < 0) continue;
    for (const v of m[2].matchAll(/(--[\w-]+):\s*(#[0-9a-fA-F]{6})\s*;/g)) out[v[1]] = v[2];
  }
  return out;
}

test('text and the result mark keep their contrast through every color-vision deficiency', () => {
  for (const mode of ['paper', 'ink']) {
    const t = tokens(mode);
    for (const k of ['--text', '--muted', '--accent', '--bg', '--surface', '--land']) assert.ok(t[k], `${mode} ${k} is a hex token`);
    for (const [kind, m] of Object.entries(MACHADO)) {
      const see = (k) => simulate(toLinear(t[k]), m);
      for (const fg of ['--text', '--muted', '--accent', '--caution', '--danger'].filter((k) => t[k])) {
        for (const bg of ['--bg', '--surface']) {
          const c = ratio(see(fg), see(bg));
          assert.ok(c >= 4.5, `${mode}, ${kind}: ${fg} on ${bg} is ${c.toFixed(2)}:1`);
        }
      }
      for (const fill of ['--land', '--bg']) {
        const c = ratio(see('--accent'), see(fill));
        assert.ok(c >= 3, `${mode}, ${kind}: the result mark over ${fill} is ${c.toFixed(2)}:1`);
      }
    }
  }
});

test('the simulation is the published one: gray stays gray, pure hues shift as documented', () => {
  const gray = toLinear('#808080');
  for (const m of Object.values(MACHADO)) simulate(gray, m).forEach((v, i) => assert.ok(Math.abs(v - gray[i]) < 0.01));
  // Protanopia darkens red: pure red loses most of its luminance.
  assert.ok(lum(simulate(toLinear('#ff0000'), MACHADO.protanopia)) < lum(toLinear('#ff0000')));
});
