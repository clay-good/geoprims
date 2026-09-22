// Global keyboard shortcuts (web/command-palette "Global shortcuts"). Single-key
// shortcuts act only outside text fields and can be turned off (WCAG 2.1.4);
// Ctrl/Cmd+K always opens the palette.
export const SHORTCUTS = [
  ['/', 'Open the command palette'],
  ['Ctrl+K or ⌘K', 'Open the command palette, even in a text field'],
  ['?', 'Show these shortcuts'],
  ['g then h', 'Go to the home page'],
  ['u', 'Switch to the next unit profile'],
  ['m', 'Mute or unmute sounds, once audio is on'],
  ['c', 'Switch the map between flat and globe'],
  ['y', 'Copy the result as JSON'],
  ['l', 'Copy the link to this calculation'],
  ['s', 'Swap points A and B, on tools with two points'],
  ['p', 'Play or pause the scene, on tools with one'],
  ['[ and ]', 'Previous or next tool in this group'],
  ['Esc', 'Close the palette or this list'],
  ['↑ ↓ or Ctrl+N Ctrl+P', 'Move through palette results'],
  ['Enter', 'Open the selected result'],
  ['Ctrl+Enter or ⌘Enter', 'Open the selected result in a new tab'],
  ['> in the palette', 'List actions: display modes, erase local data, and more'],
];

/** What a page shortcut says when this page cannot do it. */
export const UNAVAILABLE = {
  canvas: 'This page has no map.',
  'copy-json': 'There is no result to copy on this page.',
  'copy-link': 'There is no calculation to link to on this page.',
  swap: 'This tool has no points A and B to swap.',
  play: 'This tool has no scene to play.',
};

const KEY = 'gp-single-keys';

export function singleKeysOn() {
  try {
    return localStorage.getItem(KEY) !== 'off';
  } catch {
    return true;
  }
}

export function setSingleKeys(on) {
  try {
    if (on) localStorage.removeItem(KEY);
    else localStorage.setItem(KEY, 'off');
  } catch {
    /* private mode: lasts for this page */
  }
}

const typing = (el) => typeof el?.closest === 'function' && !!el.closest('input, textarea, select, [contenteditable]:not([contenteditable=false])');

/**
 * Decides what a keydown does: 'palette', 'sheet', 'home', or null. `pending`
 * carries a "g" pressed within the last second. Pure, so it can be tested.
 */
export function shortcutFor(e, { singleKeys = true, pending = false } = {}) {
  const mod = e.ctrlKey || e.metaKey;
  if (e.key.toLowerCase() === 'k' && mod && !e.altKey && !e.shiftKey) return 'palette';
  if (!singleKeys || mod || e.altKey || typing(e.target)) return null;
  if (e.key === '/') return 'palette';
  if (e.key === '?') return 'sheet';
  if (pending && e.key === 'h') return 'home';
  if (e.key === 'g') return 'pending';
  if (e.key === 'u') return 'units';
  // Page shortcuts: the tool page acts on them if it can.
  if (e.key === 'm') return 'mute';
  const page = { c: 'canvas', y: 'copy-json', l: 'copy-link', s: 'swap', p: 'play', '[': 'previous', ']': 'next' }[e.key];
  return page ?? null;
}

let sheet;
export function openSheet() {
  if (!sheet) {
    sheet = document.createElement('dialog');
    sheet.className = 'palette shortcuts';
    sheet.setAttribute('aria-labelledby', 'shortcuts-title');
    const h = document.createElement('h2');
    h.id = 'shortcuts-title';
    h.textContent = 'Keyboard shortcuts';
    const dl = document.createElement('dl');
    for (const [k, what] of SHORTCUTS) {
      const dt = document.createElement('dt');
      dt.textContent = k;
      const dd = document.createElement('dd');
      dd.textContent = what;
      dl.append(dt, dd);
    }
    const note = document.createElement('p');
    note.className = 'notice';
    note.textContent = 'Single-key shortcuts can be turned off from the palette: type "> shortcuts".';
    const close = document.createElement('button');
    close.type = 'button';
    close.textContent = 'Close';
    close.addEventListener('click', () => sheet.close());
    sheet.append(h, dl, note, close);
    document.body.append(sheet);
  }
  sheet.showModal();
}

/** Says something briefly in a polite live region (the shortcuts' feedback). */
export function say(text) {
  let toast = document.querySelector('.toast');
  if (!toast) {
    toast = document.createElement('p');
    toast.className = 'toast card';
    toast.setAttribute('role', 'status');
    document.body.append(toast);
  }
  toast.textContent = text;
  toast.hidden = false;
  clearTimeout(toast.timer);
  toast.timer = setTimeout(() => (toast.hidden = true), 2500);
}

/** `m`: mutes or unmutes, and only once the reader has turned audio on. */
async function toggleMute() {
  const { audioMuted, audioOn, setAudioMuted } = await import('./sound.js');
  if (!audioOn()) return say('Sounds are off. Turn them on in Settings.');
  setAudioMuted(!audioMuted());
  say(audioMuted() ? 'Audio muted' : 'Audio on');
}

/** Switches to the next unit profile and says which, in a polite live region. */
async function cycleUnits() {
  const { nextProfile, PROFILES, setProfile } = await import('./prefs.js');
  const next = nextProfile();
  setProfile(next);
  say(`Units: ${PROFILES.find(([id]) => id === next)[1]}`);
}

export function wireKeys(palette) {
  let pendingUntil = 0;
  document.addEventListener('keydown', (e) => {
    const what = shortcutFor(e, { singleKeys: singleKeysOn(), pending: Date.now() < pendingUntil });
    if (!what) return;
    if (what === 'pending') {
      pendingUntil = Date.now() + 1000;
      return;
    }
    e.preventDefault();
    pendingUntil = 0;
    if (what === 'palette') palette();
    else if (what === 'sheet') openSheet();
    else if (what === 'home') location.href = '/';
    else if (what === 'units') cycleUnits();
    else if (what === 'mute') toggleMute();
    else if (what === 'previous' || what === 'next') {
      const to = document.querySelector('[data-sibling]')?.dataset[what];
      if (to) location.href = to;
      else say('This is the only tool in its group.');
    } else {
      // The page acts and cancels the event; if nothing did, say why.
      const handled = !dispatchEvent(new CustomEvent('gp-shortcut', { detail: what, cancelable: true }));
      if (!handled) say(UNAVAILABLE[what]);
    }
  });
}
