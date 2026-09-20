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

The first command runs at the repository root and builds the Wasm modules and catalog into `dist/`. The web build copies them into `public/`.

| Path | What |
|---|---|
| `src/pages/` | Home, domain hubs, group hubs, and one page per tool id (route map in `contracts/routes-and-urls`) |
| `src/components/ToolApp.svelte` | The tool island: status phrases with a mark and the cited threshold; schema-driven form (list inputs edit as one row per line, comma- or tab-separated), live answer card (core-rendered `display` and `summary`), warnings ordered by severity, copy actions, permalinks (`#v1:` via the core `link` module), Clear and Try the example |
| `src/components/PageHeader.astro`, `ListFilter.astro`, `ToolCards.astro` | The page-template building blocks: breadcrumbs, `h1`, and purpose line; the list filter (`?q=`, count, no-match search); and the tool card grid |
| `src/lib/prefs.js`, `src/pages/settings/` | What this browser remembers: the unit profile, the number format, Field mode (larger targets, larger answers, step buttons), and the recent and pinned lists — all in local storage, nothing sent anywhere |
| `src/lib/compute.worker.js` | Runs the Wasm modules off the main thread; stale results are dropped |
| `src/lib/report.js`, `sw/sw.js` | The problem-report payload and the dialog's opening and send states, shared with the Worker's validator; the service worker precaches the release and lets every `/api/` request past it, so a report is never cached, replayed, or queued |
| `src/styles/global.css` | Atlas design tokens: paper (the default) and ink, and the Geist fonts |
| `scripts/lastmod.mjs` | The content hash behind each sitemap `lastmod`: a page's words with Astro's scoped classes and bundle hashes taken out, so a JavaScript-only release moves no dates |
| `scripts/routes.mjs` | The route-map gate: classifies every built page against `contracts/routes-and-urls` and fails naming anything outside the map, then writes `dist/_redirects` from `data/redirects.json` and from every deprecated tool in the catalog |
| `test/build.test.mjs` | One page per endpoint, answer in the HTML, canonical and noindex rules, no third-party requests |
| `src/lib/quality.mjs` | The monthly correctness summary behind `/quality/`, derived only from the known-issues file and the changelog |
| `src/lib/licenses.mjs` | What `/licenses/` renders: the asset registry's rows with their attribution, the build dependencies, and the datasets left out with reasons |
| `src/lib/head.mjs` | The one source for every page's title and description: the 60- and 155-character caps, the qualifier-drop rule, and the repeated-title and superlative lints |
| `src/lib/notices.mjs` | Ranks a tool page's notes, including the simplified-method banner a manifest declares as `x-limitation`, by the contract's priority and splits them into the two shown in full and the rest behind "N more notes" |
| `test/i18n.test.mjs` | No physical writing direction in any stylesheet, and a shared message lives in the catalog rather than being written out twice |
| `test/compute.test.mjs` | Stale results: an out-of-order reply is dropped, the newest wins, and edits are debounced |
| `test/worker.test.mjs` | The browser compute worker driven as the page drives it: every message gets exactly one envelope back |
| `src/lib/keyboard.mjs`, `test/keyboard.test.mjs` | Lifting the sticky answer above the on-screen keyboard: what the visual viewport says is covered becomes `--keyboard` on the document, re-read whenever it moves |
| `test/responsive.test.mjs` | Layouts for every width: nothing declared wider than a 320 px screen, no column that refuses to narrow, wide tables inside a box that scrolls, and the side-by-side layout starting at a tablet |
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

Not built yet: the map canvas and animated scenes, offline packs and the pack manager, import and export, docs pages, and Playwright end-to-end suites. `node scripts/serve.mjs` serves the build with its production headers and CSP.
