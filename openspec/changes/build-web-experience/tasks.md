## 1. Site skeleton

- [x] 1.1 Scaffold the Astro site with Svelte islands in `apps/web`; verify `build` produces static HTML for a sample tool route
- [x] 1.2 Generate one route per endpoint from `catalog/v1.json`; verify route count equals the catalog endpoint count
- [x] 1.3 Configure static-host headers (CSP, security headers, immutable caching for hashed assets); verify with the header smoke test (scripts/headers.mjs writes dist/_headers and a meta CSP; test/headers.test.mjs; scripts/serve.mjs serves the build under those headers, where recompute, the palette, and the report dialog ran with zero CSP violations)
- [ ] 1.4 Add performance budgets from `contracts/reference-profiles` (LCP ≤ 2.0 s hard / 1.5 s target, interactive ≤ 2.5 s, INP ≤ 200 ms, shell JS ≤ 90 KB), measured with Playwright traces rather than the Lighthouse CLI; verify on 50 sampled routes

## 2. Design system

- [ ] 2.1 Define design tokens and the five theme modes (paper, ink, sunlight, night, high-contrast) in the Atlas style (design W9); verify the no-color-literal lint passes and each mode renders the component gallery (Atlas tokens and all five modes in global.css: paper, ink, sunlight, night with a brightness slider, and high contrast, one signal-orange accent; the mode is set before first paint from a saved choice or the OS preference; test/theme.test.mjs lints color literals, checks AA contrast for every text token in every mode and night's luminance band; pending: re-theme from the earlier hud/daylight modes to paper/ink, and the component gallery render)
- [ ] 2.2 Self-host and subset Geist Sans, Geist Mono, and icons (≤ 80 KB); verify font budget and zero third-party requests (done for fonts: Geist and Geist Mono 1.7.2 variable, subset to 52 KB with tools/codegen/subset-fonts.sh, OFL included, tested against the budget; the build test already forbids third-party requests. Pending: an icon set)
- [ ] 2.3 Build base components (field, unit selector, result value, warning, badge, table, tabs, toast, dialog) with focus and target-size rules; verify axe-core passes in every mode
- [ ] 2.4 Build the print stylesheet (`paper` palette, canvas snapshot); verify a print-to-PDF snapshot test (the paper palette applies in print from any mode; pending: expanded panels, the canvas snapshot, and the PDF test)
- [ ] 2.5 Add the contrast audit on rendered pixels (over real map fills) and color-vision-deficiency simulations; verify all states meet 4.5:1 / 3:1
- [ ] 2.6 Externalize strings into a message catalog and use logical CSS properties; verify the i18n lint passes (done: the logical-properties half. apps/web/test/i18n.test.mjs fails on any physical direction in a source or built stylesheet, in a margin, padding, border, inset, scroll offset, corner radius, or text alignment, and leaves the logical spellings alone. It found and fixed one padding-left. The catalog holds the strings shown in more than one place, and the lint fails if one is written out again instead of imported. Pending: moving the rest of the interface's strings into the catalog, which is worth doing when a second locale is)

## 3. Tool page and form engine

- [x] 3.1 Implement the schema-driven `ToolForm` (coordinates, quantities, enums, arrays, objects); verify it renders every manifest in the catalog without errors (apps/web/test/form.test.mjs walks all 205 manifests: every input has a control of the kind its schema calls for, a select for a choice and a textarea for a list, every control is labelled, a list says what one row holds and names its columns, an optional input says so, and no tool shows more than five inputs before "More options", counted the way the meta-schema counts. It found a tool whose form was empty and two that show six)
- [ ] 3.2 Implement the multi-notation coordinate field with parsed interpretation and lat/lon swap; verify the DMS and ambiguous-order scenarios
- [x] 3.3 Implement live recompute with debouncing, stale marking, and worker cancellation; verify the stale-result scenario (apps/web/test/compute.test.mjs drives the client with a stubbed worker and delivers replies out of order: a superseded reply resolves to nothing and never reaches the card, the newest always wins, and an answer, a permalink, and a search do not supersede each other because they are keyed apart. It also holds the edit debounce to a sensible window and checks an edit cancels the pending run rather than queueing another)
- [ ] 3.4 Implement the result panel (units, copy formats, provenance, warnings); verify the copy-as-agent-call scenario (done: the copy formats are a pure module, each checked against a real result. The agent call is parsed, run, and compared with the answer it claims to reproduce, and with the call the page prints for developers. It was unreachable before this, with no button and the wrong key, and both are fixed. A list output is now part of the answer rather than a count of it: a short, narrow list of rows renders as a table in the card, so a forecast shows its periods, winds aloft show their levels, and a zone lookup shows its zones. A long list stays a count, and the rule lives in src/lib/rows.js with its caps tested directly. Pending: the units switch and provenance in an expander, with 3.3)
- [ ] 3.5 Implement permalinks (`#v1:` deflate/base64url) with migration hooks; verify the round-trip and old-version scenarios (done: the core link module encodes and decodes the fragment grammar, the web app reads it on load and writes it as you type, and both scenarios are pinned: every state in data/fragment-vectors.json survives a round trip with its flags, `#example` carries no state, a `v9:` link is refused with the "newer version of geoprims" message the page shows, and a damaged fragment is refused rather than half-read. Pending: migration hooks, which have nothing to migrate until a field is renamed)
- [ ] 3.6 Implement tool chaining ("send to") with quantity-type matching and chained permalinks; verify the geodesic-to-wind scenario
- [ ] 3.7 Implement recent and pinned tools, settings screen, and erase-all-local-data; verify the recent-list and erase scenarios (done: up to 50 recent tools and pins in local storage, on the home page and in the empty palette; /settings/ with unit profile and number format applied immediately to every tool, and erase-all; `u` cycles the unit profile. Checked in a browser: three tools listed most recent first, and the aviation profile shows NM. Pending: the settings that depend on unbuilt features: coordinate display format, reduced motion, audio, canvas, and offline packs)
- [x] 3.8 Implement the safety notice for aviation, drone, and navigation tools; verify it is visible on every such route (the spec's wording in the tool header with a link to /disclaimer/; a build test checks every tool: present before the calculator on operational routes, absent elsewhere)
- [x] 3.9 Implement no-JS and no-Wasm fallbacks; verify the JavaScript-disabled and Wasm-blocked scenarios (a noscript notice above the pre-rendered worked example; with WebAssembly missing or refused by policy, the tool area says "This tool needs WebAssembly, which is disabled in this browser" with minimum browsers, tested by running the worker with WebAssembly removed)

## 4. Command palette and keyboard

- [x] 4.1 Implement the palette (combobox pattern, `/` and Cmd/Ctrl+K, Esc focus restore); verify the open and text-field scenarios (apps/web/src/lib/palette.js, loaded on first use from a header button, `/`, or Ctrl/Cmd+K; arrows and Ctrl+N/P, Enter, Ctrl/Cmd+Enter for a new tab, a polite result count; checked in a browser, including `/` typing normally in a field. Tab-to-pin waits for 3.7)
- [ ] 4.2 Implement uFuzzy search with ranking boosts; verify the ranking fixture set (including `densty alt`, `tas`, `wca`) and the 16 ms latency benchmark (the core ranker replaces uFuzzy so the palette and `geoprims_search` rank identically; an interned vocabulary cut p95 at 1,000 entries from 58 ms to 1.6 ms with identical rankings on 2,182 queries; the fixture and benchmark pass. Pending: recency and pinned boosts, with 3.7)
- [x] 4.3 Implement paste-to-detect using the Wasm detector chain; verify the H3 and ambiguous-geohash scenarios (gp_detect in the search module recognizes H3, geohash, quadkey, z/x/y, Plus Code, coordinates and MGRS, METAR, and altimeter groups by syntax; the worker confirms each by running its decoder and links the tools worth opening, pre-filled. S2 and Maidenhead wait for their tools. Verified in Node and in a browser under the production CSP)
- [ ] 4.4 Implement action mode (`>`) and global shortcuts with the `?` sheet; verify each shortcut in an end-to-end test (done: `>` actions for display modes, accent, the shortcut toggle and sheet, erasing local data, and the trust pages; `/`, Ctrl/Cmd+K, `?`, and `g h`, with single-key shortcuts switchable off; unit tests for every shortcut and a browser check. Pending: `m`, `c`, `u`, `y`, `l`, `s`, `p`, `[`, `]` with the features they act on, and the end-to-end suite)

## 5. Map canvas

- [ ] 5.1 Implement the renderer abstraction on luma.gl (WebGPU → WebGL2) and the Canvas2D fallback; verify identical layer output in snapshot tests across backends (started: the 2D-canvas renderer the spec requires as the fallback, drawing every current layer; WebGPU and WebGL2 paths pending)
- [ ] 5.2 Implement 2D map projections (Web Mercator, equirectangular, polar azimuthal) and the 3D orthographic globe; verify the antimeridian and polar-cap scenarios (Web Mercator and the orthographic globe, with pan, zoom, and rotate by pointer and keyboard; antimeridian-contiguous paths and limb-clipped fills; polar azimuthal pending)
- [ ] 5.3 Implement layer kinds from the tool contract, with densification from Wasm; verify each kind with a visual regression fixture (point, geodesic and rhumb lines densified through the core, and polygons with geodesic edges; other kinds pending)
- [ ] 5.4 Implement vector-diagram mode (wind triangle, airspeed gauge, traverse sketch, cross-section, profile chart); verify fixtures for each (started: SVG diagrams from core values for the wind triangle, runway wind components, closest point of approach, and fly-by turns, each described for screen readers; the airspeed gauge, traverse sketch, cross-section, and profile chart pending)
- [ ] 5.5 Implement canvas input (click to set, drag points) with form sync and keyboard equivalents; verify the drag-waypoint scenario
- [ ] 5.6 Implement the readout overlay (cursor readout in chosen format, scale bar, true and magnetic north, projection name); verify the MGRS readout scenario
- [ ] 5.7 Implement the Natural Earth base layer (bundled 110m, on-demand 50m file) in the Atlas cartographic style; verify offline rendering, the no-tile-requests scenario, and the result-stands-out contrast scenario (started: Natural Earth 110m land, lakes, and borders as one 22 KB file from tools/codegen/basemap.py, in the Atlas style with land and graticule tokens, precached for offline use; the 50m file pending)
- [ ] 5.8 Implement animated scenes (timeline descriptor, play/pause, scrubber, speed, loop, playhead in the permalink, camera easing, path draw-in) for CPA, fly-by and holding turns, sun position, survey patterns, and route legs; verify the CPA, permalink-scrub, and reduced-motion scenarios (started: tools declare a timeline (driving input, range end, key moment) in the core manifest; the CPA scene plays, scrubs, changes speed, and loops, with every frame a core result and the playhead in the permalink; it opens paused on the CPA and never autoplays. Checked in a browser. Fly-by and holding turns, sun position, survey patterns, route legs, and camera easing on the globe pending)
- [ ] 5.9 Implement canvas export (PNG with attribution, SVG, GeoJSON); verify the attribution-footer scenario
- [x] 5.10 Implement the canvas accessible description; verify the screen-reader summary scenario (done: the canvas carries a description of what is drawn plus the result sentence, updated with each result)
- [ ] 5.11 Benchmark 100,000-vertex scenes; verify p95 frame time ≤ 16.7 ms on the reference profile

## 6. Audio

- [ ] 6.1 Implement the lazy-loaded synthesized sound module (≤ 6 KB) with the event map, rate limits, peak level, and mute/volume; verify every audio-feedback scenario in an automated test with an offline AudioContext

## 7. Import, export, and batch

- [ ] 7.1 Implement worker-based parsers for GeoJSON, KML/KMZ, GPX, CSV/TSV, WKT/WKB with size limits and safe parsing; verify with a fixture corpus including malicious KML
- [ ] 7.2 Implement the CSV column-mapping dialog with swap detection and CRS choice; verify the swapped-columns scenario
- [ ] 7.3 Implement geometry validation and repair on import; verify the unclosed-ring scenario
- [ ] 7.4 Implement exporters (JSON, GeoJSON, KML, GPX, CSV, WKT, text, calculation sheet); verify round-trip import of each exported geographic format
- [ ] 7.5 Implement batch mode (100,000 rows, progress, cancel, per-row errors, joined export); verify the 10,000-row mixed-error scenario

## 8. Offline PWA

- [x] 8.1 Add the web app manifest and Workbox service worker with a ≤ 12 MB precache; verify installability and the airplane-mode scenario (a hand-written worker instead of Workbox, to keep zero runtime dependencies: apps/web/sw/sw.js with a build-generated precache of every page, module, the catalog, and on-demand assets, 4.3 MB compressed; the build fails over 12 MB; test/pwa.test.mjs runs the worker in a simulated scope, and a manual check with the server stopped loaded and recomputed density altitude and EGM96 geoid height)
- [ ] 8.2 Implement the offline pack manager (sizes, resumable verified downloads, delete, usage and quota, persist request, Safari warning); verify the pack-size and Safari scenarios
- [ ] 8.3 Implement update prompts, version display, and cache hygiene; verify the update-prompt and superseded-pack scenarios (done: a waiting release, the "Reload to update" prompt, the footer version, and old-release deletion that keeps `gp-pack-*` caches; the footer links the changelog and verification report; pending: the pack manager's "update available" marking and asset versions in settings)
- [ ] 8.4 Implement the missing-asset-offline fallback messaging; verify the EGM2008-offline scenario

## 9. Documentation pages

- [ ] 9.1 Implement the standard docs template with build-time MathML and vector-generated worked examples; verify the build fails when an example disagrees with the tool
- [ ] 9.2 Implement SEO metadata, structured data, preview images, and sitemap; verify the unique-title lint (done: metadata from the shared head module with its caps and lints, the unique-title lint over every built page, the allow-listed JSON-LD, and the sitemap index with its lastmod ledger; pending: preview images)
- [ ] 9.3 Build domain and group index pages; verify the airspeed index scenario
- [ ] 9.4 Write the learning guides from the journey list in `plan-launch-and-value-proof` (L3) with pre-filled chains; verify each guide end to end
- [ ] 9.5 Add dated regulatory references and "Proposed" labels; verify the Part 108 scenario

## 10. Launch checks

- [ ] 10.1 Run the full end-to-end, accessibility, visual, performance, and privacy suites on a release candidate; verify all pass
- [ ] 10.2 Conduct a manual screen-reader pass (VoiceOver and NVDA) on the palette, one tool per domain, and settings; verify issues are fixed or tracked before launch

## 11. Page template

- [x] 11.1 Record the page template (`specs/web/page-template`) and point `apps/web/README.md` at it; verify every page type in the anatomy table exists
- [x] 11.2 Add the shared `PageHeader` component and use it on every page; verify each built page has exactly one `h1` (test/template.test.mjs; the home hero is its own header)
- [x] 11.3 Tool page self-service: field-located errors, click-to-copy result rows, the unit profile switch, and "Go the other way"; verify the field-error and copy scenarios (the error's JSON-pointer `field` marks the input; checked in the browser with latitude 95; related-tool reasons now read as words)
- [x] 11.4 Findable lists: filter with count and `?q=`, group jump chips, and the no-match search; verify the shareable-filter scenario (jump chips show for 2 to 12 groups; past that the filter does the job)
- [x] 11.5 Home example chips and the not-found page; verify the unknown-URL scenario (`scripts/serve.mjs` serves 404.html with status 404, as static hosts do)
