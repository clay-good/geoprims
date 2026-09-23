// The home page's pinned tools (web/app-shell "Recent and pinned tools"),
// drawn from local storage and redrawn when they change.
import { pins } from './prefs.js';

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
  if (!pinned) return;
  const draw = () => fill(pinned, pins());
  addEventListener('gp-lists', draw);
  draw();
}
