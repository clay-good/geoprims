// The small, always-loaded side of audio (web/audio-feedback). Audio is off
// until a reader turns it on; then the player (audio.js) loads, and its
// context starts on their next gesture, as browsers require. Every sound
// accompanies something already shown on screen; none is the only signal.

const read = (k, fallback) => {
  try {
    const v = localStorage.getItem(k);
    return v === null ? fallback : JSON.parse(v);
  } catch {
    return fallback;
  }
};
const write = (k, v) => {
  try {
    localStorage.setItem(k, JSON.stringify(v));
  } catch {
    /* storage blocked: lasts for this page */
  }
  globalThis.dispatchEvent?.(new Event('gp-audio'));
};

export const audioOn = () => read('gp-audio', false) === true;
export const audioMuted = () => read('gp-audio-muted', false) === true;
export const audioVolume = () => Math.max(0, Math.min(1, Number(read('gp-audio-volume', 0.8)) || 0));

let player = null;
let loading = null;
/** 'off', 'waiting' (for a gesture), 'on', or 'unavailable'. */
let state = 'off';
export const audioState = () => state;

async function start() {
  loading ??= import('./audio.js').then(async ({ createPlayer }) => {
    const p = createPlayer({ volume: audioVolume() });
    p.setMuted(audioMuted());
    const ok = await p.start();
    state = ok ? 'on' : 'unavailable';
    player = ok ? p : null;
    globalThis.dispatchEvent?.(new Event('gp-audio'));
    return player;
  });
  return loading;
}

/** Plays an event's sound, if the reader turned audio on. Never throws, never retries. */
export function sound(name) {
  if (!audioOn() || audioMuted()) return;
  player?.play(name);
}

export function setAudioOn(on) {
  write('gp-audio', on === true);
  if (!on) {
    player?.setMuted(true);
    player = null;
    loading = null;
    state = 'off';
  } else start(); // a settings toggle or palette action is itself a gesture
}

export function setAudioMuted(m) {
  write('gp-audio-muted', m === true);
  player?.setMuted(m === true);
}

export function setAudioVolume(v) {
  write('gp-audio-volume', v);
  player?.setVolume(audioVolume());
}

/** Waits for the first gesture on a page where audio is on, then starts it. */
export function wireAudio() {
  if (!audioOn()) return;
  state = 'waiting';
  const go = () => {
    removeEventListener('pointerdown', go, true);
    removeEventListener('keydown', go, true);
    start();
  };
  addEventListener('pointerdown', go, true);
  addEventListener('keydown', go, true);
}
