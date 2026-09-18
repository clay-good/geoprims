## 1. Site skeleton

- [ ] 1.1 Scaffold the Astro site with Svelte islands in `apps/web`; verify `build` produces static HTML for a sample tool route
- [ ] 1.2 Generate one route per endpoint from `catalog/v1.json`; verify route count equals the catalog endpoint count
- [ ] 1.3 Configure static-host headers (CSP, security headers, immutable caching for hashed assets); verify with the header smoke test
- [ ] 1.4 Add Lighthouse CI budgets (LCP 1.5 s, interactive 2.5 s, shell JS ≤ 90 KB); verify on 50 sampled routes

## 2. Design system

- [ ] 2.1 Define design tokens and the five theme modes (hud, daylight, sunlight, night, high-contrast); verify the no-color-literal lint passes and each mode renders the component gallery
- [ ] 2.2 Self-host and subset fonts and icons (≤ 80 KB); verify font budget and zero third-party requests
- [ ] 2.3 Build base components (field, unit selector, result value, warning, badge, table, tabs, toast, dialog) with focus and target-size rules; verify axe-core passes in every mode
- [ ] 2.4 Build the print stylesheet (`paper` palette, canvas snapshot); verify a print-to-PDF snapshot test
- [ ] 2.5 Add the contrast audit on rendered pixels (effects on) and color-vision-deficiency simulations; verify all states meet 4.5:1 / 3:1
- [ ] 2.6 Externalize strings into a message catalog and use logical CSS properties; verify the i18n lint passes

## 3. Tool page and form engine

- [ ] 3.1 Implement the schema-driven `ToolForm` (coordinates, quantities, enums, arrays, objects); verify it renders every manifest in the catalog without errors
- [ ] 3.2 Implement the multi-notation coordinate field with parsed interpretation and lat/lon swap; verify the DMS and ambiguous-order scenarios
- [ ] 3.3 Implement live recompute with debouncing, stale marking, and worker cancellation; verify the stale-result scenario
- [ ] 3.4 Implement the result panel (units, copy formats, provenance, warnings); verify the copy-as-agent-call scenario
- [ ] 3.5 Implement permalinks (`#v1:` deflate/base64url) with migration hooks; verify the round-trip and old-version scenarios
- [ ] 3.6 Implement tool chaining ("send to") with quantity-type matching and chained permalinks; verify the geodesic-to-wind scenario
- [ ] 3.7 Implement recent and pinned tools, settings screen, and erase-all-local-data; verify the recent-list and erase scenarios
- [ ] 3.8 Implement the safety notice for aviation, drone, and navigation tools; verify it is visible on every such route
- [ ] 3.9 Implement no-JS and no-Wasm fallbacks; verify the JavaScript-disabled and Wasm-blocked scenarios

## 4. Command palette and keyboard

- [ ] 4.1 Implement the palette (combobox pattern, `/` and Cmd/Ctrl+K, Esc focus restore); verify the open and text-field scenarios
- [ ] 4.2 Implement uFuzzy search with ranking boosts; verify the ranking fixture set (including `densty alt`, `tas`, `wca`) and the 16 ms latency benchmark
- [ ] 4.3 Implement paste-to-detect using the Wasm detector chain; verify the H3 and ambiguous-geohash scenarios
- [ ] 4.4 Implement action mode (`>`) and global shortcuts with the `?` sheet; verify each shortcut in an end-to-end test

## 5. HUD canvas

- [ ] 5.1 Implement the renderer abstraction on luma.gl (WebGPU → WebGL2) and the Canvas2D fallback; verify identical layer output in snapshot tests across backends
- [ ] 5.2 Implement 2D map projections (Web Mercator, equirectangular, polar azimuthal) and the 3D orthographic globe; verify the antimeridian and polar-cap scenarios
- [ ] 5.3 Implement layer kinds from the tool contract, with densification from Wasm; verify each kind with a visual regression fixture
- [ ] 5.4 Implement vector-diagram mode (wind triangle, airspeed gauge, traverse sketch, cross-section, profile chart); verify fixtures for each
- [ ] 5.5 Implement canvas input (click to set, drag points) with form sync and keyboard equivalents; verify the drag-waypoint scenario
- [ ] 5.6 Implement the HUD overlay (cursor readout in chosen format, scale bar, true and magnetic north, projection name); verify the MGRS readout scenario
- [ ] 5.7 Implement the bundled Natural Earth basemap and the optional self-hosted vector basemap with attribution and zoom ≤ 7 whole-tile fetch cap; verify offline rendering and the tile-zoom cap
- [ ] 5.8 Implement HUD post-processing effects with reduced-motion and settings gates; verify the reduced-motion scenario and the flash-rate check
- [ ] 5.9 Implement canvas export (PNG with attribution, SVG, GeoJSON); verify the attribution-footer scenario
- [ ] 5.10 Implement the canvas accessible description; verify the screen-reader summary scenario
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

- [ ] 8.1 Add the web app manifest and Workbox service worker with a ≤ 12 MB precache; verify installability and the airplane-mode scenario
- [ ] 8.2 Implement the offline pack manager (sizes, resumable verified downloads, delete, usage and quota, persist request, Safari warning); verify the pack-size and Safari scenarios
- [ ] 8.3 Implement update prompts, version display, and cache hygiene; verify the update-prompt and superseded-pack scenarios
- [ ] 8.4 Implement the missing-asset-offline fallback messaging; verify the EGM2008-offline scenario

## 9. Documentation pages

- [ ] 9.1 Implement the standard docs template with build-time MathML and vector-generated worked examples; verify the build fails when an example disagrees with the tool
- [ ] 9.2 Implement SEO metadata, structured data, preview images, and sitemap; verify the unique-title lint
- [ ] 9.3 Build domain and group index pages; verify the airspeed index scenario
- [ ] 9.4 Write the five learning guides with pre-filled chains; verify each guide end to end
- [ ] 9.5 Add dated regulatory references and PROPOSED labels; verify the Part 108 scenario

## 10. Launch checks

- [ ] 10.1 Run the full end-to-end, accessibility, visual, performance, and privacy suites on a release candidate; verify all pass
- [ ] 10.2 Conduct a manual screen-reader pass (VoiceOver and NVDA) on the palette, one tool per domain, and settings; verify issues are fixed or tracked before launch
