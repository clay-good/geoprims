// The light/dark control in the site header (web/visual-theme "Theme modes").
// The mode itself is applied before first paint by the inline script in
// Base.astro; this labels the button with the mode it switches to and saves an
// explicit choice in this browser only.
const SUN = '<circle cx="12" cy="12" r="4" /><path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41" />';
const MOON = '<path d="M21 12.79A9 9 0 1 1 11.21 3a7 7 0 0 0 9.79 9.79z" />';

export function wireTheme(button) {
  if (!button) return;
  const root = document.documentElement;
  // The button offers the mode you are not in, and says so.
  const paint = () => {
    const next = root.dataset.theme === 'ink' ? 'paper' : 'ink';
    const word = next === 'ink' ? 'Dark' : 'Light';
    button.querySelector('.label').textContent = word;
    button.querySelector('svg').innerHTML = next === 'ink' ? MOON : SUN;
    button.setAttribute('aria-label', `Switch to ${word.toLowerCase()} mode`);
    button.title = `Switch to ${word.toLowerCase()} mode`;
  };
  button.addEventListener('click', () => {
    root.dataset.theme = root.dataset.theme === 'ink' ? 'paper' : 'ink';
    try {
      localStorage.setItem('gp-theme', root.dataset.theme);
    } catch {
      /* private mode: the choice lasts for this page */
    }
    paint();
  });
  paint();
}
