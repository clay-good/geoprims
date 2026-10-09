// Loads one domain's diagrams on demand, so a tool page carries only its own
// drawings rather than every diagram on the site. The build renders the first
// diagram into the page; this draws the later ones as the reader edits.

const PARTS = {
  aviation: () => import('./diagrams/aviation.js'),
  drone: () => import('./diagrams/drone.js'),
  geodesy: () => import('./diagrams/geodesy.js'),
  navigation: () => import('./diagrams/navigation.js'),
  raster: () => import('./diagrams/raster.js'),
  survey: () => import('./diagrams/survey.js'),
  time: () => import('./diagrams/time.js'),
};

/** A drawing function for one tool, `(args, result, at, view) => diagram | null`, or null. */
export async function loadDiagram(id) {
  const part = PARTS[id.split('.')[0]];
  if (!part) return null;
  const [m, kit] = await Promise.all([part(), import('./diagrams/kit.js')]);
  const f = m.DIAGRAMS[id];
  return f ? (args, result, at = '', view = {}) => kit.draw(f, args, result, at, view) : null;
}
