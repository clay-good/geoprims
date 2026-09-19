// Global keyboard shortcuts (web/command-palette "Global shortcuts"). Single-key
// shortcuts act only outside text fields and can be turned off (WCAG 2.1.4);
// Ctrl/Cmd+K always opens the palette.
export const SHORTCUTS = [
  ['/', 'Open the command palette'],
  ['Ctrl+K or ⌘K', 'Open the command palette, even in a text field'],
  ['?', 'Show these shortcuts'],
  ['g then h', 'Go to the home page'],
  ['Esc', 'Close the palette or this list'],
  ['↑ ↓ or Ctrl+N Ctrl+P', 'Move through palette results'],
  ['Enter', 'Open the selected result'],
  ['Ctrl+Enter or ⌘Enter', 'Open the selected result in a new tab'],
  ['> in the palette', 'List actions: display modes, erase local data, and more'],
];

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
  return null;
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
  });
}
