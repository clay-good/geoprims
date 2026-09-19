// The command palette (web/command-palette): a WAI-ARIA combobox over the core
// search, loaded on first use. `/` or Ctrl/Cmd+K opens it, arrows or
// Ctrl+N/Ctrl+P move, Enter opens, Ctrl/Cmd+Enter opens in a new tab, and Esc
// closes it and returns focus to where it was.
import { search } from './compute.js';

const LIMIT = 8;
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
      aria-label="Search tools" placeholder="Search tools: density altitude, utm, knots…" autocomplete="off" spellcheck="false" enterkeyhint="go" />
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
    ...results.map((r, i) => {
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
      return li;
    }),
  );
  input.setAttribute('aria-expanded', String(results.length > 0));
  if (active >= 0) {
    input.setAttribute('aria-activedescendant', `palette-opt-${active}`);
    list.children[active]?.scrollIntoView({ block: 'nearest' });
  } else input.removeAttribute('aria-activedescendant');
}

async function update() {
  const query = input.value.trim();
  if (!query) {
    results = [];
    active = -1;
    status.textContent = '';
    return render();
  }
  const out = await search({ query, limit: LIMIT, includeExperimental: true });
  if (!out || query !== input.value.trim()) return; // superseded by a newer keystroke
  results = out.ok ? out.result.results : [];
  active = results.length ? 0 : -1;
  render();
  status.textContent = results.length ? `${results.length} ${results.length === 1 ? 'tool' : 'tools'} found` : 'No tools found';
}

function move(by) {
  if (!results.length) return;
  active = (active + by + results.length) % results.length;
  render();
}

function go(i, newTab) {
  const r = results[i];
  if (!r) return;
  if (newTab) window.open(route(r.id), '_blank', 'noopener');
  else {
    dialog.close();
    location.href = route(r.id);
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

export function openPalette() {
  if (!dialog) build();
  if (dialog.open) return input.focus();
  restore = document.activeElement;
  dialog.showModal();
  input.select();
  input.focus();
}
