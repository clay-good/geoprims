// The egress privacy test (platform/privacy-and-security). Inputs never leave
// the device, except inside a problem report the user previews and sends. This
// checks both halves: what the shipped code is able to send at all, and that a
// sentinel a user typed cannot reach a payload they did not agree to.
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { buildPayload, reportText } from '../src/lib/report.js';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const dist = join(web, 'dist');

/** Ways a page can reach the network. Only `fetch` is allowed, same-origin. */
const FORBIDDEN = ['sendBeacon', 'XMLHttpRequest', 'WebSocket', 'EventSource', 'importScripts', 'navigator.connection'];

function scripts(dir, out = []) {
  for (const f of readdirSync(dir)) {
    const p = join(dir, f);
    if (statSync(p).isDirectory()) scripts(p, out);
    else if (f.endsWith('.js')) out.push(p);
  }
  return out;
}

const bundled = () => [...scripts(join(dist, '_astro')), join(dist, 'sw.js')];

test('the shipped code has no way to reach the network but same-origin fetch', () => {
  const problems = [];
  for (const file of bundled()) {
    const src = readFileSync(file, 'utf8');
    for (const bad of FORBIDDEN) {
      if (src.includes(bad)) problems.push(`${file.slice(dist.length)} uses ${bad}`);
    }
  }
  assert.deepEqual(problems, []);
});

test('every fetch in the bundle names a same-origin path', () => {
  const problems = [];
  for (const file of bundled()) {
    const name = file.slice(dist.length);
    const src = readFileSync(file, 'utf8');
    for (const m of src.matchAll(/fetch\(\s*([`'"])([^`'"]*)\1/g)) {
      const target = m[2];
      // A template with an interpolation still has to start at the site root.
      if (!target.startsWith('/')) problems.push(`${name} fetches ${target}`);
    }
  }
  assert.deepEqual(problems, []);
});

test('the only request that sends anything is the problem report', () => {
  const posts = [];
  for (const file of bundled()) {
    const src = readFileSync(file, 'utf8');
    for (const m of src.matchAll(/fetch\(\s*([`'"])([^`'"]*)\1\s*,\s*\{([^}]*)/g)) {
      if (/method:\s*[`'"]POST/.test(m[3])) posts.push({ file: file.slice(dist.length), target: m[2] });
    }
  }
  assert.deepEqual(posts.map((p) => p.target), ['/api/reports'], JSON.stringify(posts));
});

// ------------------------------------------------------------------ sentinels

const SENTINEL_LAT = 12.345678;
const SENTINEL_NOTE = 'ZZQQ-SENTINEL-NOTE-9187';
const SENTINEL_PATH = '/aviation/altimetry/density-altitude/#v1:ZZQQSENTINELFRAGMENT';

const tool = {
  id: 'aviation.altimetry.density-altitude',
  version: '1.0.0',
  coreVersion: '0.1.0',
  buildHash: '0123456789abcdef',
  inputs: { properties: { elevation: { title: 'Field elevation', 'x-unit': 'ft' }, secret: { title: 'Secret', 'x-private': true } } },
  outputs: { properties: { density_altitude: { title: 'Density altitude' } } },
};
const result = { ok: true, result: { density_altitude: { value: 7932, unit: 'ft' } }, meta: { warnings: [], assets: [] } };
const display = { theme: 'light', unitProfile: 'default', viewportClass: 'phone' };

test('a sentinel the user typed reaches only the report they previewed', () => {
  const args = { elevation: SENTINEL_LAT, secret: SENTINEL_LAT };
  const sent = buildPayload({ tool, args, result, includeInputs: true, note: SENTINEL_NOTE, kind: 'wrong-result', display, pagePath: SENTINEL_PATH });
  const text = JSON.stringify(sent);
  // What the user agreed to send: the note, and the inputs they left included.
  assert.ok(text.includes(SENTINEL_NOTE), 'the note is sent');
  assert.ok(text.includes(String(SENTINEL_LAT)), 'the included input is sent');
  // What is never sent, even so: an input the manifest marks private.
  const privateRows = sent.inputs.filter((r) => r.field === 'secret');
  assert.deepEqual(privateRows, [], 'a private input was included');
  assert.ok(sent.outputs.every((r) => r.value === '(withheld)'), 'outputs must be withheld beside a private input');
});

test('excluding the inputs leaves no sentinel anywhere in the payload', () => {
  const args = { elevation: SENTINEL_LAT };
  const sent = buildPayload({ tool, args, result, includeInputs: false, note: '', kind: 'wrong-result', display, pagePath: SENTINEL_PATH });
  const text = JSON.stringify(sent);
  assert.ok(!text.includes(String(SENTINEL_LAT)), `the input leaked: ${text}`);
  assert.ok(!text.includes('ZZQQSENTINELFRAGMENT'), 'the permalink fragment leaked');
  assert.ok(!text.includes(SENTINEL_NOTE), 'a note leaked');
  // The copy-report text a user pastes carries the same payload, no token.
  assert.ok(!reportText(sent).includes(String(SENTINEL_LAT)));
});

test('the payload carries nothing that identifies a person or a device', () => {
  const sent = buildPayload({ tool, args: { elevation: 5000 }, result, includeInputs: true, note: 'x', kind: 'wrong-result', display, pagePath: '/x/' });
  const forbidden = ['userAgent', 'language', 'timezone', 'screen', 'ip', 'email', 'name', 'id', 'session', 'cookie'];
  for (const key of Object.keys(sent)) {
    assert.ok(!forbidden.includes(key), `the payload has a ${key} field`);
  }
  // The display class is a bucket, never a pixel size.
  assert.ok(['phone', 'tablet', 'desktop'].includes(sent.display.viewportClass));
});
