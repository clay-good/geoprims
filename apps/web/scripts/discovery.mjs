#!/usr/bin/env node
// Post-build discovery files (add-seo-and-discoverability 3.2 and 3.4), written
// into dist/ from the build's own catalog and the MCP golden surface file:
// /llms.txt, /AGENTS.md, /.well-known/mcp.json, /robots.txt, and the sitemap
// index with one sitemap per domain plus one for hubs and trust pages. Only
// indexable pages are listed. lastmod comes from a committed ledger keyed by a
// hash of each page's <main> content, so it changes only when the content does.
import { existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { contentHash } from './lastmod.mjs';
import { buildDate } from '../src/lib/build-date.mjs';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const dist = join(web, 'dist');
const SITE = 'https://geoprims.com';
const REPO = 'https://github.com/clay-good/geoprims';
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const surface = JSON.parse(readFileSync(join(root, 'mcp/surface.json'), 'utf8'));
const mcpPkg = JSON.parse(readFileSync(join(root, 'mcp/package.json'), 'utf8'));
const LEDGER = join(root, 'data/seo/lastmod.json');
const today = buildDate();

const DOMAIN_TITLES = {
  geodesy: 'Geodesy', navigation: 'Navigation', geometry: 'Geometry', aviation: 'Aviation', drone: 'Drone',
  survey: 'Survey', indexing: 'Spatial indexing', raster: 'Raster', time: 'Time', units: 'Units',
};
const domains = [...new Set(catalog.tools.map((t) => t.domain))].sort();
const { counts } = catalog;

// ---------------------------------------------------------------- llms.txt

// The explainers, read from their Markdown frontmatter: title and one-line summary.
const learnDir = join(web, 'src/content/learn');
const explainers = readdirSync(learnDir)
  .filter((f) => f.endsWith('.md'))
  .map((f) => {
    const text = readFileSync(join(learnDir, f), 'utf8');
    const field = (k) => new RegExp(`^${k}: (.+)$`, 'm').exec(text)?.[1].trim() ?? '';
    return { slug: f.slice(0, -3), title: field('title'), summary: field('summary') };
  })
  .sort((a, b) => a.title.localeCompare(b.title));

const llms = `# geoprims

> Exact, cited geospatial and aerospace calculators: geodesy, navigation, aviation, drone, surveying, spatial indexing, time, and units. The website and a local MCP server run the same Rust → WebAssembly core and return byte-identical results.

Everything is computed on the user's device. There are no accounts, ads, analytics, or cookies, and inputs never leave the device except in a problem report the user reviews and sends.

This build has ${counts.all.operations} operations and ${counts.all.endpoints} tool ids (${counts.stable.endpoints} stable, ${counts.experimental.endpoints} experimental). Agents should not scrape these pages to compute: run the MCP server instead.

## For agents

- [Use with agents](${SITE}/agents/): setup for Claude Code, Claude Desktop, VS Code, Cursor, and Windsurf
- [MCP server setup](${REPO}/tree/main/mcp#readme): clone a release and run \`node mcp/server.mjs\` (stdio, no dependencies, no network)
- [Tool catalog](${SITE}/catalog/v1.json): every tool id with its input and output schemas, units, accuracy, references, and worked example
- [MCP discovery document](${SITE}/.well-known/mcp.json)
- [AGENTS.md](${SITE}/AGENTS.md): how to run tools, relay caveats, and prepare problem reports
- [Methodology](${SITE}/methodology/): how results are checked, and what is not verified yet
- [Sources](${SITE}/sources/): every cited standard, model, and regulation with its edition and last check
- [Known issues](${SITE}/known-issues/)

## Domains

${domains.map((d) => `- [${DOMAIN_TITLES[d] ?? d}](${SITE}/${d}/): ${catalog.tools.filter((t) => t.domain === d).length} tool ids`).join('\n')}

## Learn

${explainers.map((e) => `- [${e.title}](${SITE}/learn/${e.slug}/): ${e.summary}`).join('\n')}
`;

// ---------------------------------------------------------------- AGENTS.md

const agents = `# AGENTS.md: using geoprims from an agent

geoprims has two surfaces: this website for people and a local MCP server for agents. Both run the same core and return byte-identical results. Do not scrape these pages to compute; run the server.

## Run the MCP server

\`\`\`bash
git clone --branch v${mcpPkg.version} --depth 1 ${REPO}.git
\`\`\`

\`\`\`bash
node geoprims/mcp/server.mjs
\`\`\`

It speaks MCP over stdio, needs Node 22 or newer, has no dependencies, and makes no network requests. Setup for common clients: ${REPO}/tree/main/mcp#readme

## Tool ids

Ids are \`domain.group.operation\`, like \`aviation.altimetry.density-altitude\`. Find them with \`geoprims_search\`, read inputs and units with \`geoprims_describe\`, and run with \`geoprims_run\`. Omit \`args\` to run a tool's worked example. Numbers may carry units as strings, like \`"145 kts"\`. The full catalog is at ${SITE}/catalog/v1.json.

## Relay the caveats

Every result has a \`summary\` sentence, sometimes a \`comparison\` line framing the answer against a rule of thumb or a typical range, and a \`meta\` object with \`warnings\`, \`model\`, \`accuracy\`, the data \`assets\` used, and \`limitation\` when the tool simplifies the governing method. Show the warnings to the user; they carry assumptions (for example dry air, ISA temperature, or a nominal fuel density) and limits. Results are planning and engineering aids, not certified for navigation, and not legal survey determinations.

## Report a wrong result

Call \`geoprims_report_problem\` with the tool id, the arguments, what you observed, and the expected value with its source. It sends nothing: it returns a link that reopens the tool with the report form filled in, for the user to review and send.
`;

// ---------------------------------------------------------------- mcp.json

const mcpJson = {
  name: mcpPkg.mcpName,
  title: 'geoprims',
  description: mcpPkg.description,
  version: mcpPkg.version,
  transport: 'stdio',
  homepage: SITE,
  repository: REPO,
  install: [
    { kind: 'git-clone', command: `git clone --branch v${mcpPkg.version} --depth 1 ${REPO}.git && node geoprims/mcp/server.mjs` },
  ],
  tools: surface.tools.map((t) => t.name),
  resources: surface.resources.map((r) => r.uri),
  resourceTemplates: surface.resourceTemplates.map((r) => r.uriTemplate),
};

// ---------------------------------------------------------------- sitemaps

function htmlFiles(dir, out = []) {
  for (const f of readdirSync(dir)) {
    const p = join(dir, f);
    if (statSync(p).isDirectory()) htmlFiles(p, out);
    else if (f === 'index.html') out.push(p);
  }
  return out;
}

const pages = htmlFiles(dist)
  .map((file) => ({ file, path: file.slice(dist.length).replace(/index\.html$/, '') || '/' }))
  .map((p) => ({ ...p, html: readFileSync(p.file, 'utf8') }))
  .filter((p) => !/<meta name="robots" content="noindex"/.test(p.html))
  .filter((p) => {
    const canon = /<link rel="canonical" href="([^"]+)"/.exec(p.html)?.[1];
    return canon === SITE + p.path; // self-canonical only
  });

const ledger = existsSync(LEDGER) ? JSON.parse(readFileSync(LEDGER, 'utf8')) : {};
const nextLedger = {};
for (const p of pages) {
  const hash = contentHash(p.html);
  const prev = ledger[p.path];
  nextLedger[p.path] = prev && prev.hash === hash ? prev : { hash, lastmod: today };
}
const sorted = Object.fromEntries(Object.entries(nextLedger).sort(([a], [b]) => a.localeCompare(b)));
mkdirSync(join(root, 'data/seo'), { recursive: true });
writeFileSync(LEDGER, JSON.stringify(sorted, null, 2) + '\n');

const sitemapOf = (paths) =>
  `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${paths
    .map((p) => `  <url><loc>${SITE}${p}</loc><lastmod>${sorted[p].lastmod}</lastmod></url>`)
    .join('\n')}\n</urlset>\n`;
const groupsOf = new Map([['site', []]]);
for (const p of pages.map((x) => x.path).sort()) {
  const d = p.split('/')[1];
  // Explainers and journeys share the learn sitemap (search-pages, "Sitemaps").
  const key = domains.includes(d) ? d : d === 'learn' || d === 'journeys' ? 'learn' : 'site';
  if (!groupsOf.has(key)) groupsOf.set(key, []);
  groupsOf.get(key).push(p);
}
mkdirSync(join(dist, 'sitemaps'), { recursive: true });
const index = [];
for (const [name, paths] of groupsOf) {
  if (!paths.length) continue;
  writeFileSync(join(dist, 'sitemaps', `${name}.xml`), sitemapOf(paths));
  const last = paths.map((p) => sorted[p].lastmod).sort().at(-1);
  index.push(`  <sitemap><loc>${SITE}/sitemaps/${name}.xml</loc><lastmod>${last}</lastmod></sitemap>`);
}
const sitemapIndex = `<?xml version="1.0" encoding="UTF-8"?>\n<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${index.join('\n')}\n</sitemapindex>\n`;

// ---------------------------------------------------------------- write

writeFileSync(join(dist, 'llms.txt'), llms);
writeFileSync(join(dist, 'AGENTS.md'), agents);
mkdirSync(join(dist, '.well-known'), { recursive: true });
writeFileSync(join(dist, '.well-known/mcp.json'), JSON.stringify(mcpJson, null, 2) + '\n');
writeFileSync(join(dist, 'sitemap-index.xml'), sitemapIndex);
writeFileSync(join(dist, 'robots.txt'), `User-agent: *\nAllow: /\n\nSitemap: ${SITE}/sitemap-index.xml\n`);
console.log(`discovery: llms.txt, AGENTS.md, .well-known/mcp.json, robots.txt, ${index.length} sitemaps (${pages.length} indexable pages)`);
