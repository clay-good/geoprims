// The display-mode picker (web/visual-theme). The mode itself is applied before
// first paint by the inline script in Base.astro; this keeps the picker in
// step and saves an explicit choice in this browser only.
const save = (k, v) => {
  try {
    localStorage.setItem(k, v);
  } catch {
    /* private mode: the choice lasts for this page */
  }
};

export function wireDisplay(form) {
  if (!form) return;
  const d = document.documentElement;
  const { theme, accent, dim } = form.elements;
  const sync = () => {
    theme.value = d.dataset.theme ?? 'hud';
    accent.value = d.dataset.accent ?? 'amber';
    dim.value = getComputedStyle(d).getPropertyValue('--dim').trim() || '1';
    for (const label of form.querySelectorAll('[data-for]')) label.hidden = label.dataset.for !== theme.value;
  };
  theme.addEventListener('change', () => {
    d.dataset.theme = theme.value;
    save('gp-theme', theme.value);
    sync();
  });
  accent.addEventListener('change', () => {
    if (accent.value === 'amber') delete d.dataset.accent;
    else d.dataset.accent = accent.value;
    save('gp-accent', accent.value === 'amber' ? '' : accent.value);
  });
  dim.addEventListener('input', () => {
    d.style.setProperty('--dim', dim.value);
    save('gp-dim', dim.value);
  });
  sync();
}
