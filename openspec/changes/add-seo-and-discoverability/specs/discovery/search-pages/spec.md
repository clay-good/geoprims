## Purpose

Makes every indexable geoprims page the most useful result for its query: substantive, unique, fast, correctly described to search engines, and free of the thin or duplicated patterns that search engines penalize.

## ADDED Requirements

### Requirement: Content minimums for indexable pages
Every indexable tool page SHALL contain, in the pre-rendered HTML:
- the H1 tool name
- a one-sentence purpose statement unique to the page
- the worked example rendered as "You enter / You get", with the answer visible
- a "When to use this" paragraph
- the formula with its citation and locator
- limitations
- 3–6 related tools, each with a reason
- the last-verified date

The build SHALL fail if two indexable pages share a purpose statement or worked example, or if a page's unique text (excluding template chrome) is under 150 words.

#### Scenario: Thin page blocked
- **WHEN** a generated page has only template text and a form
- **THEN** the build fails naming the page and the missing content fields

#### Scenario: Answer before script
- **WHEN** a tool page is loaded with JavaScript disabled
- **THEN** the worked-example answer is visible in the HTML

### Requirement: Page-versus-endpoint rule
Generated conversion-pair endpoints SHALL remain tool ids, search aliases, and MCP endpoints. They SHALL get an indexable page only when listed in `data/seo/high-intent-pages.json` (at most 60 entries site-wide, each with a query-demand justification) and when they carry distinct content under the content minimums. Every other generated endpoint route SHALL render the parent tool with the pair's preset and SHALL declare `rel="canonical"` to the parent tool page. They SHALL be excluded from sitemaps.

#### Scenario: Non-listed pair
- **WHEN** a crawler requests `/geodesy/convert/utm-to-georef`
- **THEN** the page renders the coordinate converter preset to UTM → GEOREF, canonicalizes to the converter page, and is absent from the sitemap

#### Scenario: High-intent pair
- **WHEN** `dms-to-decimal-degrees` is on the high-intent list
- **THEN** it has its own indexable page with a distinct worked example and a self-canonical URL

### Requirement: Titles and descriptions from one source
One module SHALL produce every page's `<title>` and meta description. The pre-rendered page and the in-app route change SHALL use it, and a parity test SHALL compare them.
- **Title:** `<Tool name> calculator — <what it answers> | geoprims` style, at most 60 characters. When too long, the qualifier is dropped before the name is truncated.
- **Description:** leads with a verb, states the output and one differentiator (e.g. "cited to ICAO Doc 7488"), and is at most 155 characters.
- A lint SHALL reject marketing superlatives ("best", "ultimate", "#1").

#### Scenario: Head parity
- **WHEN** a user navigates in-app to a tool
- **THEN** the document title equals the pre-rendered page's title

### Requirement: Canonical URLs and state
Every indexable page SHALL declare a self-referencing canonical on its clean path. User inputs SHALL remain in the fragment (never a query string). Links that open the worked example SHALL use `#example`, which search engines ignore.

#### Scenario: Canonical on shared link
- **WHEN** a crawler fetches a permalink URL with a fragment
- **THEN** the canonical is the clean tool URL

### Requirement: Structured data allowlist
JSON-LD SHALL be limited to:
- `WebApplication` (with `isAccessibleForFree: true`, `offers.price` 0, `applicationCategory`, and no ratings or reviews) plus `BreadcrumbList` on tool pages
- `CollectionPage` with `ItemList` plus `BreadcrumbList` on hub pages
- `Article` on concept explainers
- `Dataset` only on pages offering downloadable data (test vectors, geoid tiles)

A gate SHALL fail on any other type, on invalid JSON-LD, and on unescaped `<` in JSON-LD.

#### Scenario: Banned type
- **WHEN** a template adds `FAQPage` markup
- **THEN** the structured-data gate fails

### Requirement: Open Graph images
Each indexable page SHALL have an Open Graph image rendered at build time (1200 × 630). It shows the tool name, its domain, and the worked-example answer in the site's visual style, and includes alt text. There is no runtime image generation.

#### Scenario: OG image present
- **WHEN** any indexable page is built
- **THEN** it references an existing `og:image` file of 1200 × 630 px with `og:image:alt`

### Requirement: Concept explainers and journeys
The site SHALL publish concept explainer pages (at least 25 at launch), each teaching one concept practitioners search for and linking to the tools that compute it:
- density altitude
- true vs magnetic
- grid vs ground
- geoid vs ellipsoid
- GSD
- the legal definitions of "night"
- METAR reading
- holding entries
- traverse closure
- H3 resolution choice

Each SHALL be an `Article` with a cited source list and at least one interactive embedded example. Journey pages (per `web/tool-docs` learning guides) SHALL chain tools with prefilled steps.

#### Scenario: Explainer links to tool
- **WHEN** a user reads "What is density altitude?"
- **THEN** the page embeds a live example and links to the density-altitude, pressure-altitude, and ISA tools

### Requirement: Related links and hubs
Each tool SHALL have a curated related list of 3–6 tools with a one-line reason each ("Next: convert to pressure altitude"), validated for existence, no self-links, and no duplicates. When no curated list exists, the fallback SHALL be the tool's inverse, then other tools in the same group. Domain and group hubs SHALL list tools grouped by task.

#### Scenario: Related validation
- **WHEN** a related entry names a nonexistent tool
- **THEN** the related-links gate fails

### Requirement: Sitemaps and change signals
The build SHALL emit a sitemap index with one sitemap per domain plus one for explainers and trust pages. Only indexable pages SHALL be listed. `lastmod` SHALL come from a committed ledger keyed by a content hash of each page's substantive content, so dates change only when content changes. After each production deploy, CI SHALL submit changed URLs via IndexNow. `robots.txt` SHALL allow all and reference the sitemap index.

#### Scenario: Unchanged page keeps lastmod
- **WHEN** a deploy changes only the site footer
- **THEN** no tool page's `lastmod` changes

### Requirement: Performance budgets for ranking and use
Indexable pages SHALL meet, on the reference mobile profile: LCP ≤ 2.5 s (target 1.5 s), INP ≤ 200 ms (target 100 ms), and CLS ≤ 0.1 (target 0.05). The pre-rendered answer SHALL be the LCP element or appear before it. Wasm SHALL compile in the background after first paint, and calculation SHALL run in a worker. Pages SHALL reserve space for the canvas, so it causes no layout shift.

#### Scenario: CLS from canvas
- **WHEN** the HUD canvas initializes after load
- **THEN** measured CLS stays at or below 0.05

### Requirement: Manual search measurement, no analytics
Search performance SHALL be measured only through search-engine webmaster consoles (Google Search Console and Bing Webmaster Tools), logged monthly by hand in `docs/seo-log.md`: top queries, pages, clicks, and coverage issues. No analytics script SHALL be added for SEO purposes.

#### Scenario: Monthly log
- **WHEN** a month closes
- **THEN** the log gains an entry with coverage status and the top 20 queries and pages
