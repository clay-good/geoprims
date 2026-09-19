// The home page's pinned and recent tools (web/app-shell "Recent and pinned
// tools"), drawn from local storage and redrawn when they change.
import { clearRecents, pins, recents } from './prefs.js';

function fill(section, items) {
  const list = section.querySelector('ul');
  list.replaceChildren(
    ...items.map((t) => {
      const li = document.createElement('li');
      const a = document.createElement('a');
      a.href = t.route;
      a.textContent = t.title;
      li.append(a);
      return li;
    }),
  );
  section.hidden = items.length === 0;
}

export function mountLists(root) {
  const pinned = root.querySelector('section.pinned');
  const recent = root.querySelector('section.recent');
  if (!pinned || !recent) return;
  const draw = () => {
    const p = pins();
    const seen = new Set(p.map((x) => x.id));
    fill(pinned, p);
    fill(recent, recents().filter((r) => !seen.has(r.id)));
  };
  recent.querySelector('button')?.addEventListener('click', clearRecents);
  addEventListener('gp-lists', draw);
  draw();
}
