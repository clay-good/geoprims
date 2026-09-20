## 1. Page content and rules

- [ ] 1.1 Implement content-minimum checks (unique purpose, unique example, 150-word floor, answer in HTML); verify the thin-page and answer-before-script scenarios
- [ ] 1.2 Create `data/seo/high-intent-pages.json` (≤ 60, each justified) and the canonical-to-parent rendering for other pairs; verify both page-versus-endpoint scenarios
- [ ] 1.3 Implement the shared title/description module with caps, a superlative lint, and SPA parity; verify the head-parity scenario
- [ ] 1.4 Implement canonical rules and `#example` links; verify the shared-link scenario
- [x] 1.5 Implement the JSON-LD allowlist gate with escaping; verify the banned-type scenario

## 2. Content

- [ ] 2.1 Write at least 25 concept explainers with citations and embedded live examples; verify the density-altitude explainer scenario
- [ ] 2.2 Author curated related lists with reasons for all stable tools, plus the validation gate; verify the related-validation scenario
- [ ] 2.3 Build domain and group hubs grouped by task; verify hub JSON-LD and links

## 3. Build outputs

- [ ] 3.1 Render OG images at build with content-hash caching; verify the OG-image scenario
- [ ] 3.2 Emit the sitemap index per domain with the lastmod content-hash ledger; verify the unchanged-page scenario (built: a sitemap index with one sitemap per domain plus one for site pages, only self-canonical indexable pages, and lastmod from the committed ledger data/seo/lastmod.json keyed by a hash of each page's <main>, stable across rebuilds and across a build that only renames island bundles, which used to move every tool page's date; test/lastmod.test.mjs pins the unchanged-page scenario and fails when the committed ledger is stale. Pending: IndexNow submission after a production deploy)
- [ ] 3.3 Add IndexNow submission of changed URLs after production deploys; verify on a staging key
- [ ] 3.4 Generate `/llms.txt`, `/.well-known/mcp.json`, and `/AGENTS.md` from the catalog and MCP surface; verify the counts and surface-parity scenarios (built: all three from the build catalog and mcp/surface.json by apps/web/scripts/discovery.mjs, plus robots.txt; pending: links to /methodology and /sources, which do not exist yet)
- [x] 3.5 Add the "For developers and agents" block per tool; verify the copyable-call scenario

## 4. Performance

- [ ] 4.1 Ensure the pre-rendered answer is or precedes LCP, with background Wasm compile, worker compute, and reserved canvas space; verify the CLS scenario and LCP/INP budgets on 50 sampled pages

## 5. Natural-language prefill

- [x] 5.1 Implement the core query parser (normalize, rank, extract quantities, map slots); verify the density-altitude and parity scenarios
- [x] 5.2 Add slot definitions to manifests with a validation gate; verify the invalid-slot scenario
- [x] 5.3 Implement the ambiguity UI; verify the two-temperatures scenario
- [x] 5.4 Build the 500-query fixture and accuracy gate; verify the regression scenario (557 questions generated from every worked example plus 19 hand-written ones; the Search Console log does not exist yet, so real phrasings get added as it fills)
- [x] 5.5 Add `prefill` to MCP `geoprims_search` results; verify parity with the web palette

## 6. Measurement

- [ ] 6.1 Verify geoprims.com in Google Search Console and Bing Webmaster Tools, create `docs/seo-log.md`, and record the first monthly entry; verify the monthly-log scenario
