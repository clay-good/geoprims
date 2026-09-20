// Local preferences (web/app-shell "Settings", "Recent and pinned tools"):
// the unit profile, number format, recent tools (up to 50), and pinned tools.
// Everything stays in this browser's local storage; nothing is sent anywhere.
// Setting changes fire "gp-prefs" (open tools re-run with them); list changes
// fire "gp-lists" (the home page and palette redraw).

export const PROFILES = [
  ['', 'Each tool’s own units'],
  ['aviation', 'Aviation (NM, kt, ft, inHg)'],
  ['aviation-hpa', 'Aviation with hPa'],
  ['us-customary', 'US customary (mi, mph, ft, °F)'],
  ['si', 'SI (m, m/s, °C)'],
  ['survey-us', 'US survey (US survey feet)'],
  ['survey-metric', 'Survey metric (m)'],
];
export const NUMBER_FORMATS = [
  ['decimal-point', '1,234.5'],
  ['decimal-comma', '1.234,5'],
];
export const MAX_RECENT = 50;

const read = (k, fallback) => {
  try {
    const v = localStorage.getItem(k);
    return v === null ? fallback : JSON.parse(v);
  } catch {
    return fallback;
  }
};
const write = (k, v, event) => {
  try {
    localStorage.setItem(k, JSON.stringify(v));
  } catch {
    /* storage blocked: lasts for this page */
  }
  globalThis.dispatchEvent?.(new Event(event));
};

export const profile = () => read('gp-profile', '');
/** Field mode: larger targets, larger result text, and per-field step buttons. */
export const fieldMode = () => read('gp-field-mode', false) === true;
export const numberFormat = () => read('gp-number-format', 'decimal-point');
export const setProfile = (p) => write('gp-profile', p, 'gp-prefs');
export const setNumberFormat = (f) => write('gp-number-format', f, 'gp-prefs');
export function setFieldMode(on) {
  write('gp-field-mode', on === true, 'gp-prefs');
  applyFieldMode();
}

/** Puts the setting on the document, where the size tokens read it. */
export function applyFieldMode() {
  const root = globalThis.document?.documentElement;
  if (!root) return;
  if (fieldMode()) root.dataset.field = 'on';
  else delete root.dataset.field;
}

/** The next profile in the list, for the `u` shortcut. */
export const nextProfile = (p = profile()) => PROFILES[(PROFILES.findIndex(([id]) => id === p) + 1) % PROFILES.length][0];

/** `options` for a tool call, or undefined when every preference is the default. */
export function toolOptions() {
  const o = {};
  if (profile()) o.profile = profile();
  if (numberFormat() !== 'decimal-point') o.numberFormat = numberFormat();
  return Object.keys(o).length ? o : undefined;
}

/** Most recent first: [{id, title, route}]. */
export const recents = () => read('gp-recent', []);
export const pins = () => read('gp-pins', []);

export function recordUse(tool) {
  const entry = { id: tool.id, title: tool.title, route: tool.route };
  write('gp-recent', [entry, ...recents().filter((r) => r.id !== tool.id)].slice(0, MAX_RECENT), 'gp-lists');
}

export const isPinned = (id) => pins().some((p) => p.id === id);

export function togglePin(tool) {
  const now = pins();
  write('gp-pins', isPinned(tool.id) ? now.filter((p) => p.id !== tool.id) : [...now, { id: tool.id, title: tool.title, route: tool.route }], 'gp-lists');
}

export const clearRecents = () => write('gp-recent', [], 'gp-lists');

/** Erases every geoprims setting, list, and offline copy on this device, then reloads. */
export async function eraseLocalData() {
  if (!confirm('Erase settings, recent and pinned tools, and offline copies saved on this device? Nothing is stored anywhere else.')) return;
  try {
    for (const k of Object.keys(localStorage)) if (k.startsWith('gp-')) localStorage.removeItem(k);
  } catch {
    /* storage blocked: nothing saved */
  }
  for (const k of (await globalThis.caches?.keys()) ?? []) if (k.startsWith('gp-')) await caches.delete(k);
  for (const r of (await navigator.serviceWorker?.getRegistrations()) ?? []) await r.unregister();
  location.reload();
}
