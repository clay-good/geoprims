// The command palette (web/command-palette): a WAI-ARIA combobox over the core
// search, loaded on first use. `/` or Ctrl/Cmd+K opens it, arrows or
// Ctrl+N/Ctrl+P move, Enter opens, Ctrl/Cmd+Enter opens in a new tab, and Esc
// closes it and returns focus to where it was. A pasted value (an H3 cell,
// geohash, tile, Plus Code, MGRS, coordinates, METAR, or altimeter group) is
// detected first, with the tools worth opening pre-filled with it. Starting
// the query with ">" lists actions instead of tools. A question with numbers
// ("density altitude 5000 ft 30C 29.80") offers the top tool filled in.
import { detect, search } from './compute.js';
import { openSheet, setSingleKeys, singleKeysOn } from './keys.js';
import { clearRecents, eraseLocalData, pins, PROFILES, recents, setProfile } from './prefs.js';

const LIMIT = 8;

const MODES = [
  ['paper', 'Paper (light)'],
  ['ink', 'Ink (dark)'],
  ['sunlight', 'Sunlight (outdoors)'],
  ['night', 'Night (dark-adapted)'],
  ['high-contrast', 'High contrast'],
];

/** Sets a display control through its footer picker, which saves the choice. */
function setDisplay(name, value) {
  const control = document.querySelector(`form.display [name=${name}]`);
  if (!control) return;
  control.value = value;
  control.dispatchEvent(new Event(name === 'dim' ? 'input' : 'change'));
}

const page = (title, href, words) => ({ title, summary: href, words, run: () => (location.href = href) });

/** The palette's actions, rebuilt each time so their labels reflect current settings. */
function actions() {
  const on = singleKeysOn();
  return [
    ...MODES.map(([mode, label]) => ({ title: `Display: ${label}`, summary: 'Change the display mode', words: `theme mode display color colour dark light ${mode}`, run: () => setDisplay('theme', mode) })),
    { title: 'Show keyboard shortcuts', summary: 'Or press ?', words: 'help keys keyboard shortcuts', run: openSheet },
    { title: on ? 'Turn single-key shortcuts off' : 'Turn single-key shortcuts on', summary: '/, ?, and g h; Ctrl+K always works', words: 'keys keyboard shortcuts single', run: () => setSingleKeys(!on) },
    { title: 'Erase all local data', summary: 'Settings, recent and pinned tools, and offline copies on this device', words: 'erase clear reset delete storage offline cache privacy data', run: eraseLocalData },
    ...PROFILES.map(([id, label]) => ({ title: `Units: ${label}`, summary: 'Unit profile for every tool', words: `units unit profile ${id}`, run: () => setProfile(id) })),
    { title: 'Clear recent tools', summary: 'Pinned tools stay', words: 'clear recent history', run: clearRecents },
    page('Open settings', '/settings/', 'settings preferences options units'),
    page('Go to methodology', '/methodology/', 'how checked verification'),
    page('Go to changelog', '/changelog/', 'changes history results'),
    page('Go to sources', '/sources/', 'references standards citations'),
    page('Go to known issues', '/known-issues/', 'bugs problems'),
    page('Go to disclaimer', '/disclaimer/', 'safety legal'),
  ];
}
const route = (id) => '/' + id.split('.').join('/') + '/';
let dialog, input, list, status;
let results = [];
let active = -1;
let restore = null;

function build() {
  dialog = document.createElement('dialog');
  dialog.className = 'palette';
  dialog.setAttribute('aria-label', 'Search tools');
  dialog.innerHTML = `
    <input type="search" role="combobox" aria-expanded="false" aria-controls="palette-list" aria-autocomplete="list"
      aria-label="Search tools" placeholder="Search tools, paste a value, or type > for actions" autocomplete="off" spellcheck="false" enterkeyhint="go" />
    <ul id="palette-list" role="listbox" aria-label="Tools"></ul>
    <p class="palette-keys" aria-hidden="true"><kbd>↑</kbd><kbd>↓</kbd> move · <kbd>Enter</kbd> open · <kbd>Ctrl</kbd>+<kbd>Enter</kbd> new tab · <kbd>Esc</kbd> close</p>
    <p class="sr-only" role="status" aria-live="polite"></p>`;
  document.body.append(dialog);
  input = dialog.querySelector('input');
  list = dialog.querySelector('ul');
  status = dialog.querySelector('[role=status]');
  input.addEventListener('input', update);
  input.addEventListener('keydown', onKey);
  list.addEventListener('mousedown', (e) => e.preventDefault()); // keep focus in the field
  list.addEventListener('click', (e) => {
    const li = e.target.closest('[role=option]');
    if (li) go(Number(li.dataset.i), e.ctrlKey || e.metaKey);
  });
  dialog.addEventListener('close', () => {
    restore?.focus?.();
    restore = null;
  });
  dialog.addEventListener('click', (e) => {
    if (e.target === dialog) dialog.close(); // a click on the backdrop
  });
}

function render() {
  list.replaceChildren(
    ...results.flatMap((r, i) => {
      const rows = [];
      if (r.head) {
        const head = document.createElement('li');
        head.setAttribute('role', 'presentation');
        head.className = 'palette-detected';
        const strong = document.createElement('strong');
        strong.textContent = r.head;
        head.append(strong, ` ${r.headDetail}`);
        rows.push(head);
      }
      const li = document.createElement('li');
      li.id = `palette-opt-${i}`;
      li.setAttribute('role', 'option');
      li.setAttribute('aria-selected', String(i === active));
      li.dataset.i = i;
      const title = document.createElement('span');
      title.className = 'palette-title';
      title.textContent = r.title;
      li.append(title);
      if (r.stability === 'experimental') {
        const badge = document.createElement('span');
        badge.className = 'badge';
        badge.textContent = 'Experimental';
        li.append(' ', badge);
      }
      const summary = document.createElement('span');
      summary.className = 'palette-summary';
      summary.textContent = r.summary;
      li.append(summary);
      rows.push(li);
      return rows;
    }),
  );
  input.setAttribute('aria-expanded', String(results.length > 0));
  if (active >= 0) {
    input.setAttribute('aria-activedescendant', `palette-opt-${active}`);
    document.getElementById(`palette-opt-${active}`)?.scrollIntoView({ block: 'nearest' });
  } else input.removeAttribute('aria-activedescendant');
}

async function update() {
  const query = input.value.trim();
  if (!query) {
    // Pinned tools first, then recent ones.
    const pinned = pins();
    const seen = new Set(pinned.map((p) => p.id));
    const recent = recents().filter((r) => !seen.has(r.id)).slice(0, LIMIT);
    results = [
      ...pinned.map((p, k) => ({ ...p, summary: 'Pinned', head: k === 0 ? 'Pinned tools.' : null, headDetail: '' })),
      ...recent.map((r, k) => ({ ...r, summary: 'Recent', head: k === 0 ? 'Recent tools.' : null, headDetail: '' })),
    ];
    active = results.length ? 0 : -1;
    status.textContent = results.length ? `${pinned.length} pinned and ${recent.length} recent tools` : '';
    return render();
  }
  if (query.startsWith('>')) {
    const words = query.slice(1).toLowerCase().split(/\s+/).filter(Boolean);
    results = actions().filter((a) => words.every((w) => `${a.title} ${a.words}`.toLowerCase().includes(w)));
    active = results.length ? 0 : -1;
    render();
    status.textContent = results.length ? `${results.length} ${results.length === 1 ? 'action' : 'actions'}` : 'No actions found';
    return;
  }
  // Values contain digits, "+", or "/"; plain words only search.
  const [out, det] = await Promise.all([
    search({ query, limit: LIMIT, includeExperimental: true }),
    /[\d+/]/.test(query) ? detect(query) : null,
  ]);
  if (!out || query !== input.value.trim()) return; // superseded by a newer keystroke
  const found = det?.ok ? det.result.found : [];
  const detected = found.flatMap((f) =>
    f.actions.map((a, k) => ({
      title: a.title,
      summary: `Opens with ${f.value}`,
      href: a.href,
      head: k === 0 ? `Detected: ${f.label}.` : null,
      headDetail: f.summary,
    })),
  );
  const tools = out.ok ? out.result.results : [];
  const filled = tools[0]?.open ?? [];
  results = [...filled, ...detected, ...tools];
  active = results.length ? 0 : -1;
  render();
  const parts = [];
  if (filled.length) parts.push(filled[0].head);
  if (found.length) parts.push(`Detected ${found.map((f) => f.label).join(', ')}`);
  parts.push(tools.length ? `${tools.length} ${tools.length === 1 ? 'tool' : 'tools'} found` : 'No tools found');
  status.textContent = parts.join('. ');
}

function move(by) {
  if (!results.length) return;
  active = (active + by + results.length) % results.length;
  render();
}

function go(i, newTab) {
  const r = results[i];
  if (!r) return;
  if (r.run) {
    dialog.close();
    return r.run();
  }
  const href = r.href ?? route(r.id);
  if (newTab) window.open(href, '_blank', 'noopener');
  else {
    dialog.close();
    location.href = href;
    // Same page, new inputs: only the hash changed, so reload to read it.
    if (href.startsWith(`${location.pathname}#`)) location.reload();
  }
}

function onKey(e) {
  const ctrl = e.ctrlKey && !e.metaKey && !e.altKey;
  if (e.key === 'ArrowDown' || (ctrl && e.key === 'n')) move(1);
  else if (e.key === 'ArrowUp' || (ctrl && e.key === 'p')) move(-1);
  else if (e.key === 'Enter') go(active, e.ctrlKey || e.metaKey);
  else return;
  e.preventDefault();
}

/** Opens the palette, optionally with a query already typed (example chips, "search all tools"). */
export function openPalette(query = '') {
  if (!dialog) build();
  if (query) input.value = query;
  if (dialog.open) return query ? update() : input.focus();
  restore = document.activeElement;
  dialog.showModal();
  if (!query) input.select();
  input.focus();
  update();
}
