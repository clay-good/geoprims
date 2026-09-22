// Interface sounds (web/audio-feedback): off by default, each sound short and
// quiet, rate limited, the module small, and no audio file anywhere.
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { gzipSync } from 'node:zlib';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { limiter, PEAK, SOUNDS } from '../src/lib/audio.js';

const web = new URL('..', import.meta.url).pathname;

test('first visit is silent: audio is off until the reader turns it on', async () => {
  const store = new Map();
  globalThis.localStorage = { getItem: (k) => (store.has(k) ? store.get(k) : null), setItem: (k, v) => store.set(k, String(v)), removeItem: (k) => store.delete(k) };
  const s = await import('../src/lib/sound.js');
  assert.equal(s.audioOn(), false);
  assert.equal(s.audioState(), 'off');
  s.sound('computed'); // must be a silent no-op, with no player loaded
  assert.equal(s.audioState(), 'off');
});

test('the fixed event set: seven sounds, each at most 120 ms and at most -12 dBFS', () => {
  assert.deepEqual(Object.keys(SOUNDS).sort(), ['click', 'computed', 'copy', 'error', 'mode', 'palette', 'warning']);
  assert.ok(Math.abs(20 * Math.log10(PEAK) + 12) < 1e-9);
  for (const [name, s] of Object.entries(SOUNDS)) {
    assert.ok(s.dur <= 0.12, `${name}: ${s.dur} s`);
    assert.ok(s.level <= 1, `${name}: louder than the peak`);
  }
  // Error and success differ in pitch and in envelope.
  const [ok, err] = [SOUNDS.computed, SOUNDS.error];
  assert.ok(err.f0 < ok.f0 / 2 && err.wave !== ok.wave && err.dur > ok.dur);
});

test('typing fast: at most two computed sounds a second, and never more than eight sounds', () => {
  const allow = limiter();
  let computed = 0;
  for (let t = 0; t < 1000; t += 50) if (allow('computed', t)) computed++; // 20 keystrokes in a second
  assert.ok(computed <= 2, `${computed} computed sounds`);
  const burst = limiter();
  let n = 0;
  for (let i = 0; i < 20; i++) if (burst('click', i)) n++;
  assert.equal(n, 8);
  assert.equal(burst('click', 1001), true, 'a new second, a new allowance');
});

test('the audio module is at most 6 KB compressed, and the site ships no audio files', () => {
  const size = gzipSync(readFileSync(join(web, 'src/lib/audio.js'))).length;
  assert.ok(size <= 6 * 1024, `${size} bytes`);
  const files = [];
  (function walk(d) {
    for (const f of readdirSync(d)) {
      const p = join(d, f);
      if (statSync(p).isDirectory()) walk(p);
      else files.push(f);
    }
  })(join(web, 'public'));
  assert.deepEqual(files.filter((f) => /\.(mp3|wav|ogg|oga|m4a|aac|flac|opus|webm)$/i.test(f)), []);
});
