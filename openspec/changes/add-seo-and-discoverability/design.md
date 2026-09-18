## Context

Research: `docs/research/05` §2 and `docs/research/06` §2, §5, §6. Key facts:

- **Google's spam policy** (updated August 28, 2026) targets pages generated "for the primary purpose of manipulating search rankings". Generation itself is fine; pages that add nothing are the problem.
- **Structured data.** FAQ rich results stopped appearing on May 7, 2026. HowTo is deprecated. `WebApplication` stars need real reviews. MathSolver is for expression solvers on a home page, not calculators.
- **URL fragments** are ignored by Google, which suits fragment permalinks.
- **llms.txt** is not used by Google Search, but coding agents read it.
- **IndexNow** is used by Bing and others, not Google.
- **Core Web Vitals** use INP ≤ 200 ms.
- **roughlogic.com precedents:**
  - script-free shell pages with a "Run the calculator" link to `#example`
  - one head module shared between shells and the SPA, adopted after it found 1,396 of 1,804 titles drifting
  - a JSON-LD allowlist, curated related links, and a lastmod ledger
  - agent-discovery files generated from the live catalog
  - a query parser shared by site and MCP, with slot filling (`search-discovery.js`, `data/search/slots.json`)

## Goals / Non-Goals

**Goals:**
- Rank on usefulness.
- Never trip thin-content signals.
- Keep measurement manual and tracking-free.

**Non-Goals:**
- SEO experiments that require user tracking.

## Decisions

### S1. Indexable pages are curated, not enumerated
The catalog holds about 834 tool ids. Indexable pages are only:
- stable operations with full content
- up to 60 high-intent conversion pages
- explainers, hubs, journeys, and trust pages

Target at launch: about 300 indexable tool pages, plus explainers, journeys, hubs, and trust pages. The public "tools" count reports tool ids and indexable pages separately (tool-catalog amendment).

Alternative considered: one page per endpoint. Rejected because of the scaled-content risk and maintenance cost.

### S2. Pre-rendered pages carry the answer
Astro renders each page with the worked-example result, computed at build time by the same Wasm core, so the answer is in the HTML. The interactive island hydrates on top of it. This is also the no-JS and first-paint experience.

### S3. OG images at build
Rendered with Satori + resvg in the build (both MIT). A template shows the tool name, domain, and example answer. They are cached by content hash, so only changed pages re-render.

### S4. One ranker in the core
The query parser and ranker live in the Rust core, so the palette and MCP rank byte-for-byte identically. The web uses uFuzzy only for highlighting matched characters. Ranking parity is tested on the fixture.

### S5. Measurement loop
- **Monthly:** the Search Console and Bing log (queries, pages, coverage).
- **Quarterly:** a content review that promotes high-demand queries into explainers or high-intent pages, and demotes pages with no impressions after 6 months into canonicalized presets.

## Risks / Trade-offs

- **[Fewer pages means fewer long-tail entries]** → Aliases and presets still resolve in search and the palette. Explainers capture conceptual queries.
- **[OG rendering adds build time]** → Content-hash caching.
- **[Parser mis-fills a value silently]** → Only unambiguous slots are filled, every filled value is visually marked "from your query", and the fixture accuracy gate applies.

## Migration Plan

Not applicable.

## Open Questions

None that affect the specs.
