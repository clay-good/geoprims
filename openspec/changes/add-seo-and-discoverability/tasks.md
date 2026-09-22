## 1. Page content and rules

- [x] 1.1 Implement content-minimum checks (unique purpose, unique example, 150-word floor, answer in HTML); verify the thin-page and answer-before-script scenarios (done: tool pages now lay the worked example out as "You enter / You get" and say when their sources were last checked, and apps/web/test/content.test.mjs checks that every indexable page has an H1 and a purpose no other page states, shows a worked example no other page shows, and carries its answer as plain text ahead of the island that hydrates it. The 150-word floor is now in force for the pages that are indexable: every tool carries two new manifest fields, "when to use this" and "limitations", written for all 75 stable tools from what each one actually does — when to reach for it and where its answer stops — and the manifest lint refuses a stable tool that arrives without them. apps/web/test/content.test.mjs counts only the manifest's own prose, never template text, and also fails if two tools share either field or if a page does not render them. Experimental tools do not carry the fields yet, and they are the reason the floor is scoped to indexable pages)
- [x] 1.2 Create `data/seo/high-intent-pages.json` (≤ 60, each justified) and the canonical-to-parent rendering for other pairs; verify both page-versus-endpoint scenarios (the list exists with its cap and its rule — an entry needs a justification, the source that measured the demand, and the date it was read — and is deliberately empty: Search Console is not connected yet, so nothing is promoted on a guess. All 35 generated endpoints render as presets of their parent: the preset field is gone from the form, the page is noindex, the canonical is the parent, and none is in a sitemap. `tools/seo/high-intent.test.mjs` holds both branches, with the listed-page branch proved against a fixture.)
- [ ] 1.3 Implement the shared title/description module with caps, a superlative lint, and SPA parity; verify the head-parity scenario (done: apps/web/src/lib/head.mjs is the one source. Base.astro funnels every page's head through it, so the 60-character title cap and the 155-character description cap hold whatever a template passes: the title drops its qualifier before the name is shortened, and a description is cut at a sentence, then at a word. It found 3 titles and 63 descriptions over the caps, all now within them. The lint rejects repeated titles and the superlatives the build cannot prove. Pending: SPA parity, which needs in-app route changes)
- [x] 1.4 Implement canonical rules and `#example` links; verify the shared-link scenario (every tool page declares a self-referencing canonical on its clean path — a generated endpoint points at the tool it presets — the values a reader types are written only as a fragment, and a link that opens the worked example uses `#example`. `apps/web/test/canonical.test.mjs` holds all three, including that no tool page carries a query string in a link except the catalog filter's own `?q=`.)
- [x] 1.5 Implement the JSON-LD allowlist gate with escaping; verify the banned-type scenario

## 2. Content

- [ ] 2.1 Write at least 25 concept explainers with citations and embedded live examples; verify the density-altitude explainer scenario
- [x] 2.2 Author curated related lists with reasons for all stable tools, plus the validation gate; verify the related-validation scenario (every one of the 75 stable tools now points at three to six others, each with a reason. tools/trust/related.mjs fails a list that points at a tool that does not exist or at itself, repeats an entry, gives a reason outside inverse/next/alternative/parent, names an inverse that does not name it back, or runs past six, and it is checked against broken fixtures. The floor is now plain rather than a ratchet: a new stable tool has to arrive with its list)
- [x] 2.3 Build domain and group hubs grouped by task; verify hub JSON-LD and links (all 9 domain and 75 group hubs list their tools under "I want to" tasks from data/hubs.json; apps/web/test/hubs.test.mjs checks each hub's CollectionPage ItemList holds exactly its tools, its breadcrumbs end at the hub, and every internal link resolves to a built page)

## 3. Build outputs

- [ ] 3.1 Render OG images at build with content-hash caching; verify the OG-image scenario
- [ ] 3.2 Emit the sitemap index per domain with the lastmod content-hash ledger; verify the unchanged-page scenario (built: a sitemap index with one sitemap per domain plus one for site pages, only self-canonical indexable pages, and lastmod from the committed ledger data/seo/lastmod.json keyed by a hash of each page's <main>, stable across rebuilds and across a build that only renames island bundles, which used to move every tool page's date; test/lastmod.test.mjs pins the unchanged-page scenario and fails when the committed ledger is stale. Pending: IndexNow submission after a production deploy)
- [ ] 3.3 Add IndexNow submission of changed URLs after production deploys; verify on a staging key
- [x] 3.4 Generate `/llms.txt`, `/.well-known/mcp.json`, and `/AGENTS.md` from the catalog and MCP surface; verify the counts and surface-parity scenarios (built: all three from the build catalog and mcp/surface.json by apps/web/scripts/discovery.mjs, plus robots.txt, and llms.txt links /methodology, /sources, and /known-issues, which now exist; and the gate in apps/web/test/build.test.mjs: every count in llms.txt, total, stable and experimental, and per domain with its hub link, matches the build catalog, and mcp.json names exactly the golden surface's tools, resources, and templates at the server's version, or the build fails)
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
