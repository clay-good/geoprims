// Line icons for the topics, drawn on a 24-unit grid with the current text
// color, so they follow the theme and need no image files. Decorative: every
// use sits beside the topic's name, and is marked aria-hidden.

const PATHS = {
  // A globe with its graticule.
  geodesy: '<circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18M5 7.5h14M5 16.5h14"/>',
  // A compass rose.
  navigation: '<circle cx="12" cy="12" r="9"/><path d="M15.5 8.5 13.4 13.4 8.5 15.5l2.1-4.9z"/><path d="M12 3v2M12 19v2M3 12h2M19 12h2"/>',
  // A polygon with its vertices.
  geometry: '<path d="M5 17 8 6l9 2 3 9-8 4z"/><circle cx="5" cy="17" r="1.2"/><circle cx="8" cy="6" r="1.2"/><circle cx="17" cy="8" r="1.2"/><circle cx="20" cy="17" r="1.2"/><circle cx="12" cy="21" r="1.2"/>',
  // An aircraft seen from above.
  aviation: '<path d="M12 2.5c.9 0 1.5 1 1.5 2.2V9l7 4.2v2l-7-2.2v4.3l2.2 1.7v1.6L12 19.8l-3.7.8V19l2.2-1.7V13l-7 2.2v-2L10.5 9V4.7C10.5 3.5 11.1 2.5 12 2.5z"/>',
  // A quadcopter.
  drone: '<circle cx="5.5" cy="5.5" r="2.5"/><circle cx="18.5" cy="5.5" r="2.5"/><circle cx="5.5" cy="18.5" r="2.5"/><circle cx="18.5" cy="18.5" r="2.5"/><path d="M7.3 7.3 10 10M16.7 7.3 14 10M7.3 16.7 10 14M16.7 16.7 14 14"/><rect x="10" y="10" width="4" height="4" rx="1"/>',
  // A theodolite on its tripod.
  survey: '<rect x="9" y="3" width="6" height="5" rx="1"/><path d="M12 8v3M8 11h8M12 11 7 21M12 11l5 10M12 11v10"/><path d="M15 5.5h3"/>',
  // Hexagonal cells.
  indexing: '<path d="M8 3.5 12 6v4.5L8 13l-4-2.5V6zM16 3.5 20 6v4.5L16 13l-4-2.5V6zM12 11l4 2.5V18l-4 2.5L8 18v-4.5z"/>',
  // A terrain profile over a grid.
  raster: '<path d="M3 18h18M3 14h18M3 10h18" opacity=".45"/><path d="m3 17 4.5-6 3.5 3.5L15 7l6 10"/>',
  // The sun over the horizon.
  time: '<path d="M3 17h18M7 17a5 5 0 0 1 10 0"/><path d="M12 6v2M5.6 9.6l1.4 1.4M18.4 9.6 17 11M3 13h2M19 13h2"/>',
  // A scale ruler.
  units: '<rect x="2.5" y="8" width="19" height="8" rx="1"/><path d="M6 8v3M9.5 8v4.5M13 8v3M16.5 8v4.5M20 8v3"/>',
  // A magnifier, for search.
  search: '<circle cx="10.5" cy="10.5" r="6.5"/><path d="m15.5 15.5 5 5"/>',
  // A reticle, the site's mark.
  reticle: '<circle cx="12" cy="12" r="7.5"/><path d="M12 1.5v5M12 17.5v5M1.5 12h5M17.5 12h5"/><circle cx="12" cy="12" r="1.3" fill="currentColor" stroke="none"/>',
};

/** An icon's SVG, ready to set as HTML; an unknown name gives the reticle. */
export function icon(name, { size = 24, className = 'icon' } = {}) {
  const body = PATHS[name] ?? PATHS.reticle;
  return `<svg class="${className}" width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">${body}</svg>`;
}

export const ICON_NAMES = Object.keys(PATHS);
