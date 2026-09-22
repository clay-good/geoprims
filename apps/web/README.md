# apps/web

The geoprims static website: Astro 7 with Svelte 5 islands, built entirely ahead of time. Every page carries its real worked-example answer in the HTML, computed at build time by the same Wasm modules the browser loads.

**Making or changing a page? Start with the page template:** [`openspec/changes/build-web-experience/specs/web/page-template/spec.md`](../../openspec/changes/build-web-experience/specs/web/page-template/spec.md). It fixes the page anatomy, the self-service patterns, the component classes, and the wording rules, so every page feels like the same tool.

```bash
npm run build
```

```bash
npm install --prefix apps/web
```

```bash
npm run build --prefix apps/web
```

```bash
npm test --prefix apps/web
```

The browser regression tests need Chromium, Firefox, and WebKit (`npm exec --prefix apps/web -- playwright install chromium firefox webkit`):

```bash
npm run test:browser --prefix apps/web
```

To time every primary example under Chromium's 4× CPU slowdown, after building the site:

```bash
npm run bench:browser --prefix apps/web -- --output /tmp/geoprims-chromium.json
```

The benchmark runs the shared Wasm loader on a blank Chromium page because CDP does not throttle dedicated workers. It times the synchronous Wasm ABI call, excluding worker messages and asset downloads, and reports 1,000 calls per tool after 50 warm-up calls, module initialization time, and an optional `--baseline <path>` p95 comparison. CI uploads the table and JSON report.

It serves the built site with production headers, starts a long H3 polygon calculation, edits the input, and checks that the worker stops within 100 ms and the new answer appears. It also compares every live golden vector's exact serialized result in Chromium, Firefox, WebKit, and Node. A network capture checks that sentinel inputs entered on every tool page never reach a request URL, header, or body. Runtime CSP and third-party request checks cover every built route. CI installs all three browsers and runs these tests after the web build.

The first command runs at the repository root and builds the Wasm modules and catalog into `dist/`. The web build copies them into `public/`.

| Path | What |
|---|---|
| `src/pages/` | Home, domain hubs, group hubs, and one page per tool id (route map in `contracts/routes-and-urls`). `/accuracy/` explains what each tool's accuracy statement and verification results mean; it is linked with the disclaimer and privacy policy in every footer. The home and not-found search is a form that lands on `/tools/?q=` without JavaScript and opens the palette with it when there is some |
| `src/components/ToolApp.svelte` | The tool island (the tool, its worked example, Report a problem, and the actions Copy, Copy with reference, Share, and Download; no developer block): status phrases with a mark and the cited threshold; the compact diagram under the answer for a tool that declares `x-diagram-inline`; schema-driven form (list inputs edit as one row per line, comma- or tab-separated), live answer card (core-rendered `display` and `summary`), warnings ordered by severity, copy actions, permalinks (`#v1:` via the core `link` module), Clear and Try the example |
| `src/components/PageHeader.astro`, `ListFilter.astro`, `ToolCards.astro` | The page-template building blocks: breadcrumbs, `h1`, and purpose line; the list filter (`?q=`, count, no-match search); and the tool card grid |
| `src/lib/prefs.js`, `src/pages/settings/` | What this browser remembers: the unit profile, the number format, Field mode (larger targets, larger answers, step buttons), and the recent and pinned lists — all in local storage, nothing sent anywhere |
| `src/lib/compute.worker.js`, `src/lib/compute.js` | Run Wasm off the main thread; a new input interrupts a running calculation, discards its result, and replays unrelated pending calls. The answer card shows elapsed time while a call is still running |
| `src/lib/report.js`, `sw/sw.js` | The problem-report payload and the dialog's opening and send states, shared with the Worker's validator; the service worker precaches the release and lets every `/api/` request past it, so a report is never cached, replayed, or queued |
| `src/styles/global.css` | Field-instrument design tokens: light (the default, tuned for sunlight) and a night mode, the Geist fonts, and the component look — legends in small mono capitals, readouts in tabular figures, plates with registration ticks |
| `src/pages/index.astro`, `src/lib/terrain.js` | The home page: a seeded contour-terrain hero with a survey reticle and readout (decoration only; one still frame under reduced motion; paused offscreen), the search, an instrument panel of worked-example answers, and the topics |
| `src/lib/ask.js`, `src/lib/home-search.js` | One resolver for the home search and the palette: the core search ranks tools and fills the top one's inputs from the question; the home search lists matches as you type and opens the best one, values filled, on Enter, and stays a plain form without JavaScript |
| `src/lib/icons.mjs` | Line icons for the topics and the mark, drawn in the current text color, so they follow the theme and need no image files |
| `scripts/lastmod.mjs` | The content hash behind each sitemap `lastmod`: a page's words with Astro's scoped classes, island ids, hydration comments, and bundle hashes taken out, so a rebuild with unchanged content moves no dates |
| `scripts/routes.mjs` | The route-map gate: classifies every built page against `contracts/routes-and-urls` and fails naming anything outside the map, then writes `dist/_redirects` from `data/redirects.json` and from every deprecated tool in the catalog |
| `test/build.test.mjs` | One page per endpoint, answer in the HTML, canonical and noindex rules, no third-party requests |
| `src/lib/quality.mjs` | The monthly correctness summary behind `/quality/`, derived only from the known-issues file and the changelog |
| `src/lib/licenses.mjs` | What `/licenses/` renders: the asset registry's rows with their attribution, the build dependencies, and the datasets left out with reasons |
| `src/lib/head.mjs` | The one source for every page's title and description: the 60- and 155-character caps, the qualifier-drop rule, and the repeated-title and superlative lints |
| `src/lib/notices.mjs` | Ranks a tool page's notes, including the simplified-method banner a manifest declares as `x-limitation`, by the contract's priority and splits them into the two shown in full and the rest behind "N more notes" |
| `test/i18n.test.mjs` | No physical writing direction in any stylesheet, and a shared message lives in the catalog rather than being written out twice |
| `test/compute.test.mjs` | Stale results: an out-of-order reply is dropped, the newest wins, and edits are debounced |
| `test/worker.test.mjs` | The browser compute worker driven as the page drives it: every message gets exactly one envelope back |
| `test/compute-cancel.test.mjs`, `test/browser/cancellation.test.mjs` | A stuck calculation cancels within 100 ms, another request survives the worker restart, and elapsed-time updates stop after cancellation; Chromium checks cancellation during a real H3 calculation and the replacement answer |
| `test/browser/determinism.test.mjs` | Every live golden vector returns byte-identical JSON in Chromium, Firefox, WebKit, and Node, including tools that load assets |
| `test/browser/egress.test.mjs` | Chromium visits every tool page with a unique sentinel input and captures all requests, including compute-worker fetches; an injected leaking fetch proves the detector works |
| `test/browser/csp.test.mjs` | Runtime CSP and third-party request report for the 87 non-tool routes and both report-dialog states; an injected inline script is blocked, and the Turnstile script loads only after the dialog opens |
| `test/browser/asset-recovery.test.mjs` | A corrupt service-worker geoid asset is rejected and evicted; retry fetches verified bytes, which a fresh worker can use offline |
| `src/lib/keyboard.mjs`, `test/keyboard.test.mjs` | Lifting the sticky answer above the on-screen keyboard: what the visual viewport says is covered becomes `--keyboard` on the document, re-read whenever it moves |
| `test/responsive.test.mjs` | Layouts for every width: nothing declared wider than a 320 px screen, no column that refuses to narrow, wide tables inside a box that scrolls, and the side-by-side layout starting at a tablet |
| `src/lib/offline.mjs`, `test/offline.test.mjs` | The footer's offline chip: what each service-worker state may honestly claim, and never "Works offline" before the release is cached |
| `test/print.test.mjs` | The printed calculation sheet and the share payload: what the print rules drop, what they keep (including the example chip), and that a share says exactly what a copy says |
| `test/example.test.mjs` | The prefilled example on every tool page: a real answer in the HTML, the label until the first edit, and both ways out |
| `test/disclosure.test.mjs` | Progressive disclosure: no required input behind "More options", and every assumption a tool makes in place of an input stated on its page |
| `test/inline-diagram.test.mjs` | The compact picture under the answer: the manifests and the drawings agreeing on which tools are pictures, both drawings on the page, and no id shared between them |
| `src/lib/terms.mjs`, `test/terms.test.mjs` | Tap to define: the first use of an abbreviation becomes a button opening a native popover with the definition and its source, plus the help-text gate over every input |
| `src/lib/citation.mjs`, `scripts/links.mjs`, `test/citation.test.mjs` | One way of writing a citation for the page, the sheet, and the copied answer; the build pass that makes every outbound link `rel="noopener noreferrer"`; and the gate that no cited link is a home page and no sold document is called free |
| `test/canonical.test.mjs` | One canonical per page on its clean path, the reader's values only ever in the fragment, and `#example` on a link that opens the worked example |
| `src/lib/chain.mjs`, `test/chain.test.mjs` | "Send to": which tools take a value, the one link that carries it with a breadcrumb back, and the geodesic-to-wind scenario end to end |
| `src/lib/coordinate.mjs`, `test/coordinate.test.mjs` | The paste-a-coordinate field: any notation the catalog decodes, what it read shown before computing, and the swap when the order was assumed |
| `data/hubs.json`, `test/hubs.test.mjs` | Group hub pages: tools listed under "I want to…" tasks, each once, and a short guide where tools form a sequence |
| `src/lib/crs.mjs`, `test/crs.test.mjs` | Projected coordinates on import: UTM and State Plane CSVs, and GeoJSON in a WGS 84 UTM zone, converted by the core's inverse tools; other declared systems refused |
| `src/lib/map/projection.js`, `test/projections.test.mjs` | The map's projections: Web Mercator, equirectangular, polar azimuthal equidistant, and the globe, each inverting what it draws; the polar-cap scenario |
| `src/lib/map/handles.js`, `test/handles.test.mjs` | Canvas input: drag A, B, or a single point on the map (the fields follow), and click to set a single-point tool's point |
| `src/lib/map/readout.js`, `test/readout.test.mjs` | The map's cursor readout in the chosen coordinate format (degrees, DMS, MGRS, and UTM from the core), and the magnetic north indicator |
| `src/lib/canvas-export.mjs`, `test/canvas-export.test.mjs` | Canvas export: the map as PNG with the caption and every attribution in a footer strip, its layers as GeoJSON, and diagrams as standalone SVG |
| `src/lib/export.mjs`, `test/export.test.mjs` | The eight export formats and the calculation sheet, with every geographic export read back and compared point for point |
| `src/lib/import.mjs`, `src/lib/import-rows.mjs`, `test/import*.test.mjs` | Reading a file a reader brings — GeoJSON, KML, KMZ, GPX, WKT, WKB, CSV — in the worker, never fetching what it points at, repairing geometry with a report; and filling a tool's list of points from it, the one place a pair's order is decided |
| `src/lib/batch.mjs`, `src/components/BatchPanel.svelte`, `test/batch.test.mjs` | Batch mode: a CSV of cases mapped to a tool's inputs and run in the worker 1,000 rows at a time, with progress, cancel, errors by line, and a download joined to the original columns |
| `test/targets.test.mjs` | Touch targets: the 48 px token, 56 px in Field mode, one rule sizing every control from it, nothing sizing a control below it, 8 px between neighbours, and the step buttons shown only in Field mode |
| `test/fields.test.mjs` | Stepping a typed value: the unit stays, the reader's decimal separator stays, no floating-point noise |
| `test/input-contract.test.mjs` | The numeric input contract: no `type="number"`, `inputmode="decimal"` on every numeric field, autocomplete/autocorrect/spellcheck off, `enterkeyhint` of `next` or `done`, a ± toggle wherever a value can read below zero, input text at 16 px, and pinch-zoom left alone |
| `test/form.test.mjs` | Every manifest renders: a control per input, labelled, lists naming their columns, and at most five inputs before "More options" |
| `src/lib/rows.js`, `test/tables.test.mjs` | Which list outputs the answer card shows as a table: short and narrow ones, with quantities read as "119 kt" |
| `src/lib/fields.mjs` | Which inputs are numeric, which can read below zero and so get a ± toggle, and what a Field-mode step button moves a value by, unit and decimal separator kept; shared by the form and its gates |
| `test/copy.test.mjs` | Each copy format against a real result, including running the copied agent call and comparing its answer |
| `test/egress.test.mjs` | The privacy claim, checked: no network primitive but same-origin `fetch`, one POST, and sentinel values that reach only the report the user previewed |
| `test/trace.test.mjs` | "Show your work": the page renders the core's trace or, for a decoder, every coded group beside its meaning; explaining moves no number on any tool |
| `test/content.test.mjs` | Content minimums: a unique purpose and worked example per indexable page, the "You enter / You get" block, the answer as plain text, and a last-verified date |
| `test/notices.test.mjs`, `test/chrome-copy.test.mjs` | The ranking and the two-visible limit; and the copy lint that fails any chrome string made only of capitalised words |
| `test/anatomy.test.mjs` | The canonical page anatomy: every tool page's regions in the contract's order, and one report button, in the answer card |
| `test/parity.test.mjs` | Example parity: every page ships the answer to its own primary example, and so do the printed agent call and the home page's featured card |
| `test/routes.test.mjs` | The route classifier, the canonical and noindex rules per route class, and the redirects file |

Generated endpoints (like `/units/speed/kt-to-mph/`) canonicalize to their parent operation. Experimental tools are `noindex` until they have their full content.

The map canvas, animated scenes, import and export, and core offline cache are present. Offline packs, the pack manager, and dedicated docs pages remain open. The browser suite covers long-calculation cancellation in Chromium and full golden-vector parity across three browsers; mobile layout checks remain open. `node scripts/serve.mjs` serves the build with its production headers and CSP.
