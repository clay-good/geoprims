// List filter (web/page-template "Lists are findable"): narrows every
// `.tools li[data-text]` on the page as the visitor types, hides groups left
// empty, counts matches, and keeps `?q=` in the URL so a filtered list can be shared.

/** True when every word of the query appears in the text (case-insensitive). */
export const matches = (text, query) =>
  query
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean)
    .every((w) => text.includes(w));

export function mountFilter(root) {
  const input = root.querySelector('.filter input');
  const count = root.querySelector('.filter-count');
  const empty = root.querySelector('.filter-empty');
  const items = [...root.querySelectorAll('.tools li[data-text]')];
  const total = items.length;
  const apply = () => {
    const q = input.value.trim();
    let shown = 0;
    for (const li of items) {
      const ok = matches(li.dataset.text, q);
      li.hidden = !ok;
      shown += ok;
    }
    for (const g of root.querySelectorAll('[data-group]')) {
      g.hidden = !g.querySelector('li[data-text]:not([hidden])');
      const chip = root.querySelector(`.chips a[href="#${g.id}"]`);
      if (chip) chip.hidden = g.hidden;
    }
    count.textContent = q ? `${shown} of ${total} ${total === 1 ? 'tool' : 'tools'}` : '';
    empty.hidden = !q || shown > 0;
    empty.querySelector('q').textContent = q;
    empty.querySelector('[data-palette]').setAttribute('data-palette', q);
    const url = new URL(location.href);
    if (q) url.searchParams.set('q', q);
    else url.searchParams.delete('q');
    history.replaceState(null, '', url);
  };
  input.value = new URLSearchParams(location.search).get('q') ?? '';
  input.addEventListener('input', apply);
  input.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && input.value) {
      input.value = '';
      apply();
    }
  });
  empty.querySelector('.clear').addEventListener('click', () => {
    input.value = '';
    apply();
    input.focus();
  });
  if (input.value) apply();
}
