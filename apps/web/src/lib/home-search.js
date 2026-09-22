// The big search on the home page (and the not-found page). Type a tool's name
// or a question with the numbers in it — "crosswind rwy 27 wind 300 at 15" —
// and the best match is listed first; Enter opens it, with any values the
// question gave already filled in. Without JavaScript the same form lands on
// the catalog with the query in the URL.
//
// It is a WAI-ARIA combobox: arrows move, Enter opens, Esc closes the list.
import { reducedMotion } from './prefs.js';
import { ask } from './ask.js';

const DEBOUNCE_MS = 110;

/** "Open with these values" rows lead with the tool; the rest read as they are. */
function row(r, i, active) {
  const li = document.createElement('li');
  li.id = `ask-opt-${i}`;
  li.setAttribute('role', 'option');
  li.setAttribute('aria-selected', String(i === active));
  li.dataset.i = i;
  li.className = `ask-row ask-${r.kind}`;
  const title = document.createElement('span');
  title.className = 'ask-title';
  const summary = document.createElement('span');
  summary.className = 'ask-summary';
  if (r.kind === 'filled') {
    title.textContent = r.tool;
    const tag = document.createElement('span');
    tag.className = 'ask-tag';
    tag.textContent = 'Values filled';
    title.append(' ', tag);
    summary.textContent = r.summary;
  } else if (r.kind === 'detected') {
    title.textContent = r.title;
    summary.textContent = r.head ? `${r.head.replace(/\.$/, '')} · ${r.summary}` : r.summary;
  } else {
    title.textContent = r.title;
    if (r.stability === 'experimental') {
      const tag = document.createElement('span');
      tag.className = 'ask-tag muted';
      tag.textContent = 'Experimental';
      title.append(' ', tag);
    }
    summary.textContent = r.summary ?? '';
  }
  li.append(title, summary);
  return li;
}

export function mountHomeSearch(form) {
  if (!form) return;
  const input = form.querySelector('input[type=search]');
  const list = document.createElement('ul');
  list.className = 'ask-list';
  list.id = `${input.id}-list`;
  list.setAttribute('role', 'listbox');
  list.setAttribute('aria-label', 'Matching tools');
  list.hidden = true;
  form.append(list);
  const status = document.createElement('p');
  status.className = 'sr-only';
  status.setAttribute('role', 'status');
  status.setAttribute('aria-live', 'polite');
  form.append(status);
  input.setAttribute('role', 'combobox');
  input.setAttribute('aria-controls', list.id);
  input.setAttribute('aria-autocomplete', 'list');
  input.setAttribute('aria-expanded', 'false');
  input.setAttribute('enterkeyhint', 'go');

  let results = [];
  let resultsFor = '';
  let active = -1;
  let timer = 0;

  const render = () => {
    list.replaceChildren(...results.slice(0, 7).map((r, i) => row(r, i, active)));
    const open = results.length > 0 && document.activeElement === input;
    list.hidden = !open;
    input.setAttribute('aria-expanded', String(open));
    if (active >= 0) input.setAttribute('aria-activedescendant', `ask-opt-${active}`);
    else input.removeAttribute('aria-activedescendant');
  };

  const update = async () => {
    const query = input.value.trim();
    if (!query) {
      results = [];
      resultsFor = '';
      active = -1;
      return render();
    }
    const answered = await ask(query, { limit: 7 });
    if (!answered || query !== input.value.trim()) return;
    const filledId = answered.filled ? answered.results[0].id : null;
    results = answered.results.filter((r, i) => i === 0 || !(r.kind === 'tool' && r.id === filledId));
    resultsFor = query;
    active = results.length ? 0 : -1;
    render();
    status.textContent = answered.filled
      ? `${results[0].tool}, with the values from your question. Press Enter to open it.`
      : results.length
        ? `${results.length} matches. ${results[0].title} is first.`
        : 'No tool matches that yet.';
  };

  const go = async (i = active) => {
    // Enter before the results arrive still goes to the first result for
    // exactly what was typed, never to a stale one.
    if (input.value.trim() !== resultsFor) {
      clearTimeout(timer);
      await update();
    }
    const r = results[i >= 0 ? i : 0];
    if (!r) return;
    location.href = r.href;
  };

  input.addEventListener('input', () => {
    clearTimeout(timer);
    timer = setTimeout(update, DEBOUNCE_MS);
  });
  input.addEventListener('focus', render);
  input.addEventListener('blur', () => setTimeout(render, 120));
  input.addEventListener('keydown', (e) => {
    if (e.key === 'ArrowDown' && results.length) active = (active + 1) % Math.min(results.length, 7);
    else if (e.key === 'ArrowUp' && results.length) active = (active - 1 + Math.min(results.length, 7)) % Math.min(results.length, 7);
    else if (e.key === 'Enter') {
      e.preventDefault();
      go();
      return;
    } else if (e.key === 'Escape') {
      results = [];
      resultsFor = '';
      active = -1;
    } else return;
    e.preventDefault();
    render();
  });
  list.addEventListener('mousedown', (e) => e.preventDefault());
  list.addEventListener('click', (e) => {
    const li = e.target.closest('[role=option]');
    if (li) go(Number(li.dataset.i));
  });
  form.addEventListener('submit', (e) => {
    e.preventDefault();
    go();
  });

  // Example questions fill the field and search, so a first visitor sees
  // what a question with numbers does.
  for (const chip of document.querySelectorAll('[data-ask]')) {
    chip.addEventListener('click', () => {
      input.value = chip.getAttribute('data-ask');
      input.focus();
      update();
    });
  }

  // The placeholder cycles through real questions until the field is used.
  const examples = (input.dataset.examples ?? '').split('|').filter(Boolean);
  if (examples.length > 1 && !reducedMotion()) {
    let k = 0;
    const cycle = setInterval(() => {
      if (document.activeElement === input || input.value) return;
      k = (k + 1) % examples.length;
      input.placeholder = examples[k];
    }, 3200);
    input.addEventListener('input', () => clearInterval(cycle), { once: true });
  }
}
