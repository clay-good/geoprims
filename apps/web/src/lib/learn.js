// Concept explainers (discovery/search-pages, "Concept explainers and
// journeys"): one Markdown file per concept in src/content/learn/, its
// frontmatter naming the tools that compute it and the sources behind it.
// Loaded through Vite's glob at build time, so a new file is a new page.
const files = import.meta.glob('../content/learn/*.md', { eager: true });

/** Every explainer: { slug, route, ...frontmatter, Content }, in title order. */
export const EXPLAINERS = Object.entries(files)
  .map(([path, mod]) => {
    const slug = path.split('/').pop().replace(/\.md$/, '');
    const fm = mod.frontmatter;
    // YAML reads a bare date as a Date; pages and JSON-LD want YYYY-MM-DD.
    const day = (d) => (d instanceof Date ? d.toISOString() : String(d)).slice(0, 10);
    return { slug, route: `/learn/${slug}/`, ...fm, published: day(fm.published), updated: day(fm.updated ?? fm.published), Content: mod.Content };
  })
  .sort((a, b) => a.title.localeCompare(b.title));

/** The explainers that name a tool, for the tool page's "Learn the concept" links. */
export const explainersFor = (id) => EXPLAINERS.filter((e) => e.tools.includes(id));

/** The order audiences appear in on /learn/. */
export const AUDIENCES = ['Pilots', 'Drone operators', 'Surveyors', 'Developers and GIS'];
