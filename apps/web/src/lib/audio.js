// Synthesized interface sounds (web/audio-feedback). Loaded only when a reader
// turns audio on; no audio files, only oscillators, noise, and envelopes. Each
// sound lasts at most 120 ms and peaks at or below -12 dBFS at full volume.

/** -12 dBFS as a linear gain: the loudest any sound may be. */
export const PEAK = 10 ** (-12 / 20);
/** Each sound: waveform, start and end pitch (Hz), length (s), attack (s), level (of PEAK). */
export const SOUNDS = {
  click: { wave: 'noise', f0: 0, f1: 0, dur: 0.018, attack: 0.001, level: 0.5 },
  mode: { wave: 'triangle', f0: 520, f1: 780, dur: 0.07, attack: 0.004, level: 0.7 },
  computed: { wave: 'sine', f0: 880, f1: 880, dur: 0.06, attack: 0.003, level: 0.6 },
  warning: { wave: 'triangle', f0: 660, f1: 520, dur: 0.1, attack: 0.006, level: 0.85 },
  error: { wave: 'square', f0: 220, f1: 165, dur: 0.12, attack: 0.002, level: 0.5 },
  palette: { wave: 'sine', f0: 600, f1: 900, dur: 0.05, attack: 0.003, level: 0.55 },
  copy: { wave: 'sine', f0: 1320, f1: 1760, dur: 0.045, attack: 0.002, level: 0.5 },
};

/** At most 8 sounds a second, and one "computed" per 500 ms (typing fast stays calm). */
export function limiter() {
  const played = [];
  let lastComputed = -Infinity;
  return (event, now) => {
    while (played.length && now - played[0] >= 1000) played.shift();
    if (played.length >= 8) return false;
    if (event === 'computed') {
      if (now - lastComputed < 500) return false;
      lastComputed = now;
    }
    played.push(now);
    return true;
  };
}

/** Schedules one sound on `ctx` into `out`, starting at `at`; returns its source. */
export function voice(ctx, out, name, at = ctx.currentTime) {
  const s = SOUNDS[name];
  const g = ctx.createGain();
  const top = PEAK * s.level;
  g.gain.setValueAtTime(0, at);
  g.gain.linearRampToValueAtTime(top, at + s.attack);
  g.gain.exponentialRampToValueAtTime(top * 0.001, at + s.dur);
  g.gain.setValueAtTime(0, at + s.dur);
  let src;
  if (s.wave === 'noise') {
    const n = Math.ceil(ctx.sampleRate * s.dur);
    const buf = ctx.createBuffer(1, n, ctx.sampleRate);
    const d = buf.getChannelData(0);
    // A fixed pseudo-random sequence: the same click every time.
    let x = 0x2545f491;
    for (let i = 0; i < n; i++) {
      x ^= x << 13; x ^= x >>> 17; x ^= x << 5;
      d[i] = ((x >>> 0) / 0xffffffff) * 2 - 1;
    }
    src = ctx.createBufferSource();
    src.buffer = buf;
  } else {
    src = ctx.createOscillator();
    src.type = s.wave;
    src.frequency.setValueAtTime(s.f0, at);
    src.frequency.linearRampToValueAtTime(s.f1, at + s.dur);
  }
  src.connect(g).connect(out);
  src.start(at);
  src.stop(at + s.dur);
  return src;
}

/**
 * The live player: a context made on the first gesture, a master gain for
 * volume and mute, and the limiter. `start()` resolves false when the browser
 * refuses audio, and the app carries on silently.
 */
export function createPlayer({ volume = 0.8 } = {}) {
  let ctx = null;
  let master = null;
  let muted = false;
  const live = new Set();
  const allow = limiter();
  return {
    async start() {
      try {
        ctx ??= new AudioContext();
        master ??= ctx.createGain();
        master.connect(ctx.destination);
        master.gain.value = muted ? 0 : volume;
        if (ctx.state !== 'running') await ctx.resume();
        return ctx.state === 'running';
      } catch {
        return false;
      }
    },
    play(name) {
      if (!ctx || muted || ctx.state !== 'running' || !SOUNDS[name] || !allow(name, performance.now())) return false;
      const src = voice(ctx, master, name);
      live.add(src);
      src.onended = () => live.delete(src);
      return true;
    },
    setVolume(v) {
      volume = Math.max(0, Math.min(1, v));
      if (master && !muted) master.gain.setValueAtTime(volume, ctx.currentTime);
    },
    setMuted(m) {
      muted = m;
      if (!master) return;
      // Silence now: the gain drops this instant and every sounding voice stops.
      master.gain.setValueAtTime(m ? 0 : volume, ctx.currentTime);
      if (!m) return;
      for (const s of live) {
        try {
          s.stop();
        } catch {
          /* already stopped */
        }
      }
      live.clear();
    },
  };
}
