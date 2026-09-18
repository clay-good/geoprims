## Why

A public utility is only useful if people find it. The people who need a crosswind, GSD, or state plane calculator start at a search engine or, more and more, an AI assistant. They land on ad-heavy pages that often compute the wrong thing.

geoprims can outrank them by being the most useful page for each query:
- a real worked example
- cited sources
- an instant answer
- a clean mobile layout

This has to be done without tripping Google's scaled-content-abuse policy, which targets exactly the pattern of hundreds of near-duplicate generated pages (`docs/research/05` §2). It also has to be done without faking structured data.

roughlogic.com's shell pages, title rules, JSON-LD allowlist, curated related links, lastmod ledger, and agent-discovery files are a working template (`docs/research/06` §2, §5, §6).

Depends on: `build-web-experience`, `add-trust-and-proof`.

## What Changes

- **Content minimums for every indexable page:**
  - a unique purpose statement and a unique worked example
  - the formula with a citation
  - limitations, related tools, and a last-verified date
  - an answer visible in the HTML before any script runs
- **A page-versus-endpoint rule.** Generated conversion pairs stay as tool ids, search aliases, and MCP endpoints. They get their own indexable page only when they are on a short high-intent list (at most 60 site-wide) and carry distinct content. All others render the parent tool with a preset and canonicalize to it.
- **Head rules:**
  - one shared source for titles and descriptions, with length caps
  - self-canonical clean URLs; inputs stay in the fragment
  - JSON-LD allowlist: `WebApplication` with price 0 and no ratings, `BreadcrumbList`, `CollectionPage` with `ItemList`, `Dataset` only for real downloadable data
  - Open Graph images generated at build time
- **Concept explainer pages** ("What is density altitude?", "Grid vs ground distance", "Which H3 resolution?"), each linked to its tools. These carry the educational content that ranks.
- **Curated related-tool links** (3–6 per tool), domain hub pages, and practitioner journey pages.
- **Sitemaps split by domain,** with `lastmod` from a content-hash ledger. IndexNow pings from CI. Search Console and Bing Webmaster tracked by hand in a log, never with analytics.
- **Core Web Vitals budgets:** LCP ≤ 2.5 s (target 1.5 s), INP ≤ 200 ms (target 100 ms), CLS ≤ 0.1 (target 0.05). The prerendered example answer paints before Wasm loads.
- **Agent discovery:** `/llms.txt`, `/.well-known/mcp.json`, `/AGENTS.md`, and the field names each tool accepts shown on its page.
- **Natural-language prefill:** a shared query parser that extracts quantities from text such as "density altitude 5000 ft 30C 29.80". The web palette uses it to open the tool pre-filled, and MCP `geoprims_search` uses it to return pre-filled arguments.

## Capabilities

### New Capabilities

- `discovery/search-pages`: Indexable page content, metadata, structured data, linking, sitemaps, and performance for search.
- `discovery/agent-discovery`: Machine-readable site entry points for AI agents and developers.
- `discovery/natural-language-prefill`: Parsing free-text questions into a tool plus pre-filled inputs, shared by web and MCP.

### Modified Capabilities

None. This extends `web/tool-docs` and `platform/tool-catalog` (see Impact).

## Non-goals

- Paid search, backlink campaigns, or any tracking-based SEO measurement.
- FAQPage, HowTo, Review, AggregateRating, or MathSolver markup. They are either no longer shown or would misrepresent the site.
- Localized (non-English) pages in v1.

## Impact

- **Amends `platform/tool-catalog`.** Generated endpoints get indexable pages only if they are on the high-intent list, and the home page count distinguishes indexable pages from tool ids.
- **Amends `agent/mcp-server`.** `geoprims_search` gains `prefill`.
- **Adds build steps:** page generation checks, OG image rendering, sitemap and lastmod ledger, IndexNow, and agent-discovery files.
- **Adds docs:** `docs/seo.md`, the allowlist and rules, and `docs/seo-log.md`, the manual Search Console log.
