# Research: patterns to reuse from roughlogic.com

> Study of roughlogic.com (sister project: static calculator site on Cloudflare with a local MCP server), read from its `origin/main` at commit 8417a398 (2026-09-10). Paths are relative to that repository. This is the pattern source for geoprims problem reporting, SEO shells, mobile gates, citations, freshness, and MCP-from-clone.


## 1. "Report a problem" → Cloudflare D1 feedback loop

This is fully implemented, per spec-v1348 plus the spec-v1349 hardening. Files:
- `report-feedback.js` (client)
- `report-worker.mjs` (Worker)
- `migrations/0001_calculator_reports.sql`, `migrations/0002_report_attempts.sql`
- `wrangler.jsonc`
- `docs/calculator-reports.md` (runbook)
- `specs/spec-v1348.md`, `specs/spec-v1349.md`
- `scripts/check-feedback-loop.mjs` (gate)
- `test/unit/report-worker.test.js`, `test/unit/report-feedback.test.js`, `test/integration/calculator-report.test.js`

**Where the button lives.** `app.js` `renderToolView()` (around line 590) mounts exactly one `<button class="report-trigger">Report a problem</button>` into the calculator's top `headerRow`: "Back to tools" on the left, Report on the right. The button is secondary weight and 48px tall. On click it runs `import("./report-feedback.js")`, so the home view and normal calculator use never load reporting code or Turnstile. Every tile inherits the button automatically. AGENTS.md calls this the third "door", after web and MCP, and says never to fork or bypass it.

**Dialog.** A native `<dialog>` built with `createElement`/`textContent`, containing:
- Title "Report a problem".
- A disclosure: "{tool}: we will attach this calculator's URL, inputs, and results."
- A privacy line: "Do not include names, addresses, or other personal information."
- An optional textarea, "What did you expect instead?", with `maxLength=160`, a live remaining-character counter, and the placeholder "I expected about 3% voltage drop."
- An `aria-live` status line, and Cancel / "Send report" buttons.

Send stays disabled until Turnstile returns a token. Offline, the dialog shows "Reporting needs an internet connection." After a success the button reads "Report sent" and is disabled.

**Auto-attached payload.** `buildReportPayload` sends `{calculator_id, calculator_name, page_url, note, inputs:[{label,value}], outputs:{values:[{label,value}], text, truncated}, turnstile_token}`.
- Inputs: labels come from each `<label for>`; select values are the visible option text; checkboxes become "Checked" / "Not checked".
- Outputs: pulled with the same `collectOutputs()` used by Copy-all in `clipboard.js`, plus a whitespace-normalized text snapshot capped at 12,000 characters.
- `page_url` is sanitized. The query string is dropped, and only hash keys that match real non-private input ids (plus `v`) survive.
- Any control that is private (password, email, tel, file, identity-type autocomplete tokens, or `data-report-sensitive="true"`, detected by `hash-state.js isPrivateControl`) is skipped. If one exists on the tile, the output snapshot is replaced with a placeholder.
- **Not sent: the site version or build hash.** You would need to add that.

**Worker (`report-worker.mjs`).** This is a separate Worker, `roughlogic-reports`, bound only to the route `roughlogic.com/api/reports*`. The static site stays on its own Pages project. `workers_dev: false`, `preview_urls: false`, `observability.enabled: false`. There are two endpoints:
- `GET /api/reports/config` returns `{sitekey}`, with `max-age=300`. It returns 503 if anything is misconfigured, which is the kill switch.
- `POST /api/reports` checks, in order:
  1. Origin is in the `REPORT_ALLOWED_ORIGINS` allowlist.
  2. Content-Type is exactly `application/json`, and there is no Content-Encoding.
  3. `CF-Connecting-IP` is present.
  4. The body is streamed and cancelled above 24 KB, then decoded as strict UTF-8 JSON.
  5. `validateReportPayload`:
     - Top-level keys must be exactly the allowed set.
     - `calculator_id` must exist in `TOOLS`, which the Worker imports from `tools-data.js`. That is shared code with the site.
     - The server re-derives the calculator name.
     - `page_url` must be the allowed origin, with no credentials and no query string, and must identify the same calculator via `/#id` or `/tools/id/`.
     - Note ≤160, at most 100 rows, label ≤120, value ≤500.
     - A regex rejects control and bidi-override characters.
  6. A daily reporter HMAC is computed: `HMAC-SHA256(REPORT_HASH_SECRET, day + ":" + ip)`. The raw IP is never stored.
  7. Attempt-limit check.
  8. Turnstile Siteverify with `idempotency_key`, a 5-second timeout, and assertions that `action === "calculator-report"` and the hostname is allowlisted.
  9. `storeReport`.

  Accepted, duplicate and quota-dropped submissions all get the same `202 {ok:true}`, so an attacker cannot tell them apart. Security headers on the JSON response include `CSP default-src 'none'; sandbox`, `no-store`, and HSTS.

**D1 schema (0001):**
```sql
CREATE TABLE calculator_reports (
  id TEXT PRIMARY KEY NOT NULL, created_at TEXT NOT NULL,
  calculator_id TEXT NOT NULL, calculator_name TEXT NOT NULL,
  page_url TEXT NOT NULL CHECK (length(page_url) <= 8192),
  note TEXT CHECK (note IS NULL OR length(note) <= 160),
  inputs_json TEXT NOT NULL CHECK (length(inputs_json) <= 60000),
  outputs_json TEXT NOT NULL CHECK (length(outputs_json) <= 60000),
  output_text TEXT NOT NULL CHECK (length(output_text) <= 12000),
  output_truncated INTEGER NOT NULL DEFAULT 0 CHECK (output_truncated IN (0,1)),
  dedupe_key TEXT NOT NULL UNIQUE,
  status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open','resolved','wont_fix')),
  resolved_at TEXT,
  resolution_note TEXT CHECK (resolution_note IS NULL OR length(resolution_note) <= 1000));
CREATE INDEX calculator_reports_status_created ON calculator_reports (status, created_at);
CREATE INDEX calculator_reports_calculator_status ON calculator_reports (calculator_id, status);
CREATE TABLE report_limits (bucket TEXT, scope TEXT CHECK (scope IN ('global','reporter')),
  subject TEXT, count INTEGER CHECK (count >= 0), PRIMARY KEY (bucket, scope, subject)) WITHOUT ROWID;
```
Migration 0002 adds an index on `created_at` and a `report_attempt_limits` table with the same shape as `report_limits`.

**Rate limiting and spam, in layers:**
1. A zone WAF rule: 10 requests per 10 seconds per IP on `/api/reports*`, then block. This is manual Cloudflare setup, not in the repo.
2. Managed Turnstile in "interaction-only" mode. It loads only when the dialog opens, and the CSP adds `https://challenges.cloudflare.com` to `script-src` and `frame-src`.
3. Attempt caps: 10 per reporter per day and 400 per day globally.
4. Accepted-report caps: 5 per reporter per day and 200 per day globally. Environment variables cannot raise these past the hard-coded ceilings.
5. Dedupe: `dedupe_key = day + ":" + sha256(canonical payload)`, inserted with `INSERT OR IGNORE`.

All cleanup, counter increments and the conditional insert run in one `db.batch([...])` transaction. The insert is `INSERT ... SELECT ... WHERE counters < limits`, and the counters only increment `WHERE EXISTS (row with this id)`.

**Retention.** Reports are deleted after 30 days and counters after 14. A cron runs at `17 8 * * *` through `scheduled()`, and cleanup also runs before every write.

**Secrets.** `TURNSTILE_SECRET_KEY` and `REPORT_HASH_SECRET` (at least 32 characters) are declared under `"secrets": {"required": [...]}`. The sitekey is a public var. If either secret is missing, reporting is disabled (503) and the calculators keep working.

**Triage.** There is no dashboard and no public read endpoint; review goes through authenticated Wrangler. From `docs/calculator-reports.md`:
```sh
npx --no-install wrangler d1 execute roughlogic-reports --remote --command "SELECT id, created_at, calculator_id, note, page_url, inputs_json, outputs_json, output_text, output_truncated FROM calculator_reports WHERE status = 'open' ORDER BY calculator_id, created_at;"
```
Workflow: open `page_url` to reproduce, then verify against the primary source. A fix follows the normal numbered spec, regression test, worked-example fixture and full gate chain. Then run `UPDATE ... SET status='resolved', resolved_at=datetime('now'), resolution_note='Fixed in spec-vNNNN; ...'`. Use `wont_fix` only with a reason. Because D1 rows expire, the durable record is the spec, tests and CHANGELOG. Target: triage within 72 hours, 12–24 hours for credible wrong-result reports. The runbook also has a 7-step launch verification checklist.

**Parallel GitHub path.** `.github/ISSUE_TEMPLATE/wrong-answer.yml` is a form with label `correctness` and required fields: calculator/URL, inputs, answer got, answer expected, and "the published source that settles it". `config.yml` links to SECURITY.md and to the in-app button.

**Gate.** `scripts/check-feedback-loop.mjs` (in `npm run lint`) asserts:
- exactly one `report-trigger` mount inside `renderToolView`;
- the lazy `import("./report-feedback.js")`;
- the Worker, migrations, wrangler config, service-worker precache and docs are all wired;
- the three size limits agree across client, Worker and D1 CHECK constraints. This was added after they drifted apart.

**Service worker.** `sw.js` never caches `/api/*`, so the kill switch stays live.

---

## 2. SEO

**Generator.** `scripts/build.mjs` has no bundler. It copies an explicit `FILES` list and fails if any root `.js` file is left off the list. It stamps `dist/build-info.json`, patches `BUILD_HASH` in `sw.js`, and emits `llms.txt`, `/.well-known/mcp.json` and `/AGENTS.md` through `scripts/agent-discovery.mjs`. It then spawns `scripts/build-shells.mjs` (about 1,474 lines, built-ins only).

The shell generator writes:
- `dist/tools/<id>/index.html` per tile;
- `dist/groups/<slug>/index.html` per group;
- `/tools/` as the all-calculators hub;
- a 404 shell;
- `sitemap.xml`.

Shells ship no JavaScript. They carry the same CSP meta tag, `referrer no-referrer`, and `robots index,follow`.

**Titles and descriptions.** Titles use `{Name} - {Profession Noun} - Rough Logic`, capped at 70 escaped characters. The fallback order is to drop the profession noun first, then truncate the name while keeping the brand. Descriptions lead with a verb (otherwise they get a "Reference for …" prefix) and end with "Client-side, ad-free, account-free reference for {profession}." They are capped at 220 escaped characters. `shell-meta.js` is the single source for title and description, used by both the shell builder and `app.js updateHeadForTool()`. It was added after 1,396 of 1,804 SPA titles were found to differ from their shells; `test/integration/spa-head-parity.test.js` checks it.

**Canonical and social.** Each page has a canonical tag pointing at its own path URL, plus `og:type/site_name/title/description/url` and `twitter:card=summary`. **There are no OG images.** `shellHead` accepts an `ogImage` argument but nothing passes one. `theme-color` and `color-scheme` are set.

**JSON-LD.** Output is escaped (`<`, `>`, `&` become `\u` sequences). The allowlist lives in `docs/seo.md`.
- Tile pages: `[WebApplication {applicationCategory:"BusinessApplication", operatingSystem:"Any (browser)", isAccessibleForFree:true, offers:{Offer price 0 USD}, author:{Person}}, BreadcrumbList(Home > Group > Tile)]`.
- Group pages: `CollectionPage + BreadcrumbList + ItemList`.
- 404: `WebPage`.
- Banned types: FAQPage, Review, AggregateRating, and so on. `HowTo` is allowed but not emitted.

**Tile shell body** (current `origin/main`; the stale `dist` sample is older):
1. Breadcrumb, then `<h1>`, then a lead sentence.
2. A "Run the calculator" link to `/#id?example=1`, which opens the SPA preloaded with the same worked example. Tiles with no inputs get "Open the reference" instead.
3. **Example** section: "You enter / You get" lists built from `worked-examples.json`.
4. One `<details class="shell-proof">` holding detail prose, formula, edition, free-access line, governance line, "Field names used by the API: `k1`, …" (for agents), and the assumptions list.
5. A duplicate `div.shell-print-proof` that only shows in print, because a closed `<details>` does not print in WebKit.
6. Related tools, then the footer disclaimer.

**Internal linking.** `scripts/related-tiles.mjs` holds a curated `RELATED` map, a build-time-only module with 3–6 links per tile. If a tile has no entry, the fallback is the first 5 tiles in the same group. `check-related-tiles.mjs` validates it (ids exist, no self-links, no duplicates, max 6).

**Sitemap.** Home is weekly/1.0, groups monthly/0.8, tiles monthly/0.7. Per-URL `<lastmod>` comes from the committed `scripts/page-lastmod.json` ledger (a content hash plus a date) via `build-page-lastmod.mjs`, and CI checks it with `check:lastmod`. The root `sitemap.xml` in the repo is a placeholder that the build overwrites. `robots.txt` is `Allow: /` plus the Sitemap line.

**Gates.** `check-shells.mjs` checks titles and descriptions against the caps, JSON-LD parse and allowlist, presence of the citation block, a marketing-word lint, no script on shells, and gzip caps. The doc says 6 KB for tile shells; `docs/performance.md` says the group/hub cap is 68 KB. `check-shell-values.mjs` fails on NaN, undefined or empty answers. `check-discoverability.mjs` and `check-both-doors.mjs` check search reachability.

**Measurement** is Search Console and Bing Webmaster only, logged by hand in `docs/seo-log.md`. There is no analytics.

---

## 3. Mobile

- **Tokens** (`styles.css`): `--touch-min: 48px`, used as `min-height` on every control. There is one breakpoint, `@media (max-width: 760px)`, plus fluid `clamp()` type. On mobile, reference `<dl>` grids collapse to one column, and `.citation`, `.data-source-stamp` and `.limitation-banner` get `overflow-wrap:anywhere; word-break:break-word`. Wide schedules must sit inside a `.tabular-tool` wrapper that owns the horizontal scroll. SVGs use `max-width:100%`. There are rules for `prefers-reduced-motion` and `prefers-color-scheme: light`, the latter so script-free shells respect the system theme; `check-contrast` keeps the two palettes identical.
- **Input modes** (`ui-fields.js makeNumber`): `type="number"`, `inputMode="decimal"`, `autocomplete="off"`, with `label[for]` on every field. There is no submit button; compute runs on a 50 ms debounce. Each output line gets its own Copy button. A "Test with example" button is prepended to every tile.
- **Validation** (`ui-validity.js`): uses native `checkValidity()`, sets `aria-invalid`, shows the reason in a `role=status` span, and strikes through the last valid output (`.output-stale`) instead of clearing it.
- **`scripts/check-shell-mobile.mjs`** runs `npm run check:shell-mobile` after `build` in the CI integration job. It serves `dist/` (commit bc03493d switched this from `npx http-server` to the repo's own server) and asserts `scrollWidth <= clientWidth + 1`:
  - at 320×720 portrait on **every** shell;
  - at 568×320 landscape and at 375px with **200% text zoom**, on a sample of every group hub, the home shell and about 24 evenly spaced tool shells. Text zoom is emulated with an injected `html{font-size:200% !important}`.

  It skips with exit 0 if Playwright or `dist/` is missing.
- **SPA side:** `test/integration/responsive-stress.test.js` covers the full catalog at 320px, 9 routes at 200% text zoom, and a subset at 667×375, 768, 834, 759 and 761. It asserts a 2xx response first, to avoid "404 doesn't scroll" false passes. It runs on Chromium and on a `webkit-responsive` Playwright project for iOS Safari.
- `test/integration/touch-targets.test.js` measures real boxes at 390px against the 48px floor. It was added after text fields turned out to be 46px.
- `readable-type.test.js` and `answer-above-inputs.test.js` (answer placement on phones) also exist.
- `docs/mobile-responsive.md` is a running re-verification log per catalog size, with invariants: 320/375/414/760 widths, 48px targets, wrapping, inputmode, print pagination.

---

## 4. Citations and proof

**`citations.js`** (about 2.4 MB) exports `GOVERNANCE`, which holds about 25 reusable "who governs" strings (electrical, plumbing, structural, aviation, and so on). It also exports `CITATIONS[toolId]`:
```js
{ formula: string, edition: string, freeAccess: string, governance: GOVERNANCE.x,
  editionNote: string, assumptions: [{ name, value, source? }] }
```
Its render functions:
- `renderCitationBlock(parent, toolId)` mounts `.v6-reference-block` under the result as a `<dl>` with rows "Formula or table cited / Edition / source date / Public free-access pointer / What governs / Edition selector / disclosure", then "Numeric assumptions" (or "every input is user-supplied").
- `fillCitationText` turns bare domains into links using `rel="noopener noreferrer"`.
- `buildAnswerWithReference()` builds the plain-text "Copy answer with full reference block" for job logs and RFIs.

Shells print the same fields. The MCP `describe_calculator` returns the citation too.

**Coverage and consistency gates:**
- `check-citation-coverage.mjs` fails if any tile lacks a CITATIONS entry, and builds the source→tiles inverse map used for edition rollover.
- `build-citation-strings.mjs --check` checks `docs/citation-strings.generated.json`.
- `extract-citations.mjs --check` covers `test/fixtures/renderer-citations.js`.
- `check-v6-discipline`, `check-notice-variants` and `check-free-access.mjs` also run. The free-access check probes every free-access URL, runs monthly in `.github/workflows/free-access-probe.yml`, and is not part of lint.

**Freshness of standards:**
- `scripts/sources-cycle.json` has one row per standard: `{id, name, current_edition, current_release, cycle_years, next_expected, last_verified, verification_note, free_access_url, match_terms:["IMC "]}`.
- `docs/citation-freshness-ledger.md` is a table with status `current | disclosed-lag | acknowledged-stale`.
- `scripts/refresh-cadence.json` sets a per-data-folder `max_age_days`.

`check-citation-freshness.mjs` (in lint):
- **fails** when a manifest is missing `edition`/`asOf`, when a WMM-type model is past expiry, when a ledger row is missing (CF-02), or when `next_expected` has passed with no `last_verified >= next_expected` re-stamp (CF-03);
- **fails (CF-07, commit 0b7f2667)** when a manifest that names a tracked standard does not name that standard's *current* edition. This came from ICC publishing the 2027 IMC and IFGC the same day a manifest named 2024 as current. The unit test was also changed to read the current edition from the ledger instead of hard-coding it.
- **warns** when `asOf` is more than 365 days old or a model is within 6 months of expiry.

Supporting gates:
- `check-verified-on-ledger.mjs` (CF-05): a `verified_on` stamp must be backed by the ledger, not by the build clock.
- `check-data-stamp-monotonic.mjs`: provenance stamps may only move forward against the base commit, so a stale data-refresh PR cannot revert a hand-correction.
- `docs/edition-rollover.md` and `docs/edition-amendment.md` are the playbooks.

**`data-stamp.js`:** `stampDataSource(parent, {folder, shard, label})` reads `data/<folder>/manifest.json` and renders "Source: X, version V, fetched D." `stampFooterVersion()` writes "Data version: YYYY-MM-DD" from `build-info.json`.

**`limitation-banner.js`:** `renderLimitationBanner(host, {headline ≤80, replacement ≤240, who_governs ≤120, link?})` renders `<aside class="inline-notice limitation-banner" role="note">`. Canonical copy is keyed by tile, and `getLimitationCopy` is shared with MCP. The tiles that use it are flagged `SIMPLIFIED` in `tile-meta.js`; `FIELD_METER` flags tiles where the meter reading is the verdict.

**`context-band.js`:** `formatContextBand(v, lo, hi, unit)` returns `{band: low|normal|high, text: "X unit (normal; typical lo-hi unit)"}`.

**`integrity.js`:** at startup it fetches `data/integrity.json` and SHA-256s each `manifest.json`. `verifyShard` checks shard text against the manifest hashes. A mismatch shows a non-blocking banner and never refuses data. `scripts/verify-integrity.mjs` (`npm run data:verify`) and `check-integrity-coverage.mjs` are the build side.

**Correctness process** (`docs/correctness.md`, spec-v14):
- **A:** a function corpus in `docs/derivations.md` (`build-corpus.mjs`).
- **B:** worked examples in `test/fixtures/worked-examples.json`, 3,145 rows. Row shape: `{tile_id, source_publisher, source_title, source_edition_or_year, source_section_or_page, verified_by, verified_on, inputs, outputs:{key:{value, tolerance:{abs?|pct?, justification?}}}}`. They run through `worked-examples-runner.test.js`. `check-cross-validation.mjs` rejects tolerances above the ceiling unless they carry a written justification.
- **C:** a `// dims:` annotation on every export (`check-dimensions.mjs`), which also checks unit-suffixed key names like `_psi`.
- **D:** a bounds fuzzer (`check-bounds.mjs`).
- **E:** bit-pattern numerical stability tests.
- **F:** cross-tile invariants and round-trips to 1e-12.
- **G:** written derivation.
- **H:** per-group credentialed reviewer signoff in `docs/audit-trail.md`, renewed every 90 days (warn past 100, fail past 180). **H has never run: 0 of 19 groups are signed off.** The doc now says so plainly (commit 3af543b1).

`check-example-parity` and `example-parity-runtime.test.js` make sure the shell example, the SPA example button and the MCP example agree.

---

## 5. Search, navigation and the helper modules

**`tools-data.js`:** `export const TOOLS = [ { id:"wire-ampacity", name:"Wire Ampacity", group:"A", trades:["electrical"], desc:"Ampacity by gauge, conductor material, insulation rating, ambient." }, ... ]`. It has `// Group X:` section comments and is lazy-loaded (not in the home payload). It is about 398 KB gzipped against a 430 KB cap, and descriptions make up 79% of its bytes (`docs/performance.md`). `tool-modules.js` maps tile id to module/renderer and is also lazy.

**`tile-meta.js`:** per-tile `{id, group, simplified, requires_field_meter, a11y_verified_on}`, built from `SIMPLIFIED` and `FIELD_METER` sets. `check-tile-meta` warns when an axe verification is more than 180 days old.

**`search-discovery.js`** is pure and DOM-free, so it is shared with MCP:
- `resolveQuery`: exact id, then an alias.
- `matchAliasPrefix`: autocomplete.
- `normalizeQuery`: lowercases, maps `TOKEN_SYNONYMS` for unit spellings, drops `STOPWORDS`.
- `rankTools`: deterministic field-weighted scoring over name, alias, trades and desc, with prefix stemming, plural stripping and an edit-distance-1 typo fallback (`editDistance1`, Damerau).
- `promoteExactMatch`, `fallbackSearch`.
- `extractQuantities`: "120v", "150 ft", "3/4 in", negative numbers.
- `mapSlots`: fills hash params from `data/search/slots.json` rows (`{tile, slots:[{param: domInputId, units:[...]}]}`) to build pre-filled deep links.

Aliases are authored in `data/search/aliases.json` (types: industry, redirect, adjacent) and `build-alias-shards.mjs` splits them into lazily fetched per-group shards `aliases-<letter>.json`. `data/search/preview-map.json` and `data/fields/*.json` (field descriptors shared by the website and MCP `answer_query`) complete the set. Measurement scripts: `measure-ranking.mjs`, `measure-query-fill.mjs`, and `test/fixtures/queries.txt`.

**`routing.js`:** `parseHashRoute(hash, ids)` handles `#` (home), `#toolId`, `#toolId?k=v`, and `b=` bundles; unknown ids go home.

**`hash-state.js`:** `applyHashState(region, params)` sets values and dispatches input/change events. `wireHashState(region, id)` writes `#id?v=1&inputId=value` through `history.replaceState` on a 100 ms debounce. The `v=` key versions the hash schema, and an element id of `v` is reserved. Private controls are never serialized. The URL hash is the only state; `localStorage` holds just `rl-theme`. `test/integration/no-tracking.test.js` enforces this by driving real journeys and inspecting cookies, storage, IndexedDB and requests. See also `docs/hash-state.md`.

**`theme.js`:** a synchronous IIFE in `<head>` with its CSP hash pinned. It reads `rl-theme` or the system preference, sets `data-theme`, updates the `theme-color` and `color-scheme` meta tags, and wires a toggle with `aria-pressed`.

**`clipboard.js`:** `copyText` writes to the clipboard and announces "Copied" through a hidden live region. `flashCopied` animates the button. `collectOutputs` parses `<p><strong>Label:</strong><span class="out-value">`, and the report client reuses it. There is also `addCopyAllButton`.

**`ui-fields.js`:** `makeNumber`, `makeText`, `makeTextarea`, `makeSelect`, `makeCheckbox`, `makeOutputLine` (with a Copy button), `attachExampleButton`, `debounce` (50 ms), and `fmt`. Everything uses `textContent`; `innerHTML` is never used.

**`standard-sizes.js`:** `roundToStandard(v, ladder)` returns `{value, recommended, at_floor, at_cap}`. Ladders are in `STANDARD_SIZES` (breaker amps, transformer kVA, and so on).

---

## 6. MCP (`mcp/`)

- **Files:** `server.mjs` (416 lines), `catalog.mjs` (1,662 lines), `README.md`, and `package.json` (`"private": true`, `"bin": {"roughlogic-mcp": "./server.mjs"}`, `engines node>=18`, no dependencies). It is explicitly "not a publishable package" and runs from a clone.
- **Running it:** `node /abs/path/roughlogic.com/mcp/server.mjs`, or `claude mcp add roughlogic -- node /abs/path/.../mcp/server.mjs`, or a Claude Desktop/Cursor JSON entry with `command: node, args: [abs path]`. It speaks hand-rolled newline-delimited JSON-RPC 2.0 over stdio with protocol "2024-11-05" (echoing the client's version), and logs to stderr. Limits: 256 KB per message, batches of 50, 100 pending requests. The server version is read from the root `package.json`.
- **Tools:** five **meta-tools**, not one tool per calculator. All carry `readOnlyHint`, `idempotentHint`, and both `inputSchema` and `outputSchema`.
  - `search_calculators {query, trade, limit}`: with no arguments it returns a trade overview.
  - `describe_calculator {id}`: inputs with options, units and min/max; outputs; the worked example; the citation; the limitation banner; related tiles.
  - `run_calculator {id, inputs}`: raw result, rendered outputs with display strings, range warnings, and unknown-key warnings. With no inputs it runs the worked example.
  - `run_calculators {calls ≤50}`.
  - `answer_query {query}`: extracts values from free text and returns `OK`, `MISSING_INPUTS`, `NO_VALUES` or `NO_MATCH`.

  It also serves resources `roughlogic://catalog`, `roughlogic://trade/{trade}` and `roughlogic://calculator/{id}`, and prompts `find-calculator`, `run-with-inputs` and `size-and-check` (a missing required argument is an error).
- **Shared code:** `catalog.mjs` imports `tools-data.js`, `test/fixtures/compute-map.js` (id → module/export), `renderer-map.js`, `worked-examples.json`, `search-discovery.js` (the same ranker as the browser), `limitation-banner.js`, `key-labels.js`, `scripts/related-tiles.mjs`, bespoke-schema fixtures, and `calc-*.js` modules, which it lazy-imports. Tiles that depend on data shards (WMM, HUD FMR, loan limits, historical pricing) read the same JSON files from disk.
- **Gates:** `test/fixtures/mcp-surface.json` is a golden file of the tool, resource and prompt surface, checked by `mcp-surface.test.js`. Also `mcp-server.test.js`, `mcp-catalog*.test.js` and `mcp-reachability.test.js`. `scripts/check-both-doors.mjs` checks every tile against both doors: its own name must rank in the browser's top 12; every advertised input must be sendable; every example key must be advertised; and the example must run cleanly through MCP.
- **Discovery:** the build emits `dist/llms.txt` and `dist/.well-known/mcp.json` (`{name, version, transport:"stdio", install, tools, resources, prompts, homepage, source}`), with counts taken from the live catalog.

---

## 7. CI and quality gates

**`.github/workflows/ci.yml`.** Every action is pinned to a SHA, `permissions: contents: read`, `persist-credentials: false`, and `npm ci --ignore-scripts`. Jobs:
- **test:** `npm run lint`, then `test:unit`, then `data:verify`, then the "provenance stamps only move forward" check (`check:data-stamps` against the base ref, or `event.before` on a push).
- **accessibility:** `test:a11y`, an axe-core sweep of every route at 320px.
- **integration:** Chromium and WebKit. Runs `test:e2e:ci` (grep-inverted from a11y), then `build`, then `check:module-sizes`, `check:dist` (no dangling same-origin references), `check:shells`, `check:lastmod`, `check:shell-values` and `check:shell-mobile`.

**Scheduled workflows.** `data-refresh.yml` (monthly) and `data-refresh-weekly.yml` run `data:refresh`, `data:verify`, unit tests, lint and the stamp check, then `analyze-data-changes.mjs` writes a PR body, `append-source-diff-log.mjs` runs, and `peter-evans/create-pull-request` opens a PR. `free-access-probe.yml` runs monthly.

**Lighthouse was removed on 2026-08-23** (commit 88e7ea7f) because of an `@lhci/cli` advisory. `lighthouserc.json` is kept as a design target: median of 3 runs, all categories ≥0.95, FCP 1000 ms, LCP 1500 ms, TBT 100, CLS 0.05 on tool and group pages. What gates performance now:
- `check-home-payload`: home view at most 100 KB gzipped, with HTML/CSS/JS sub-budgets of 20/25/40 KB;
- `check-module-sizes`: a per-module gzip cap for each file;
- `check-shells`;
- `test/integration/perf.test.js`: Slow-3G with 4× CPU, comparing against `test/perf-baseline.json` (`{fcp_ms, lcp_ms, tbt_ms, cls, tolerance_pct:10}`) with warn and hard-fail tiers.

`check-ci-claims.mjs` makes sure README and docs describe the jobs that actually exist.

**`npm run lint`** chains about 60 scripts. Notable ones not already covered: `check-secret-files`, `check-dependency-overrides`, `check-csp` (the inline boot-script sha256 must match in both `index.html` meta and `_headers`), `check-contrast`, `check-build-hermetic`, `check-doc-links`, `check-readme-counts`, `check-community-health`, `check-ngrams` (banned phrases), `check-us-defaults`, `check-sw-precache`, `check-wiring`, `check-dead-inputs`, `check-tile-contract`, `check-renderer-schema`, and `check-tile-registries`.

**Tests.** 167 unit files in `test/unit/**` run with `node --test` and no framework. About 25 Playwright specs in `test/integration/`, including offline, service worker, print, shell-print, integrity banner, no-tracking and search-prefill.

---

## 8. Specs

This is **not OpenSpec.** It is a flat folder of about 1,685 files: `specs/spec.md` (the base), `spec-v10.md` through `spec-v1749.md`, and a few scope documents (`scope-one-box.md`, `scope-trade-expansion*.md`). Each spec inherits everything numbered before it. Format, from `spec-v1250.md`:
```
# roughlogic.com Specification v1250 -- Fresnel Zone Clearance (calc-lowvoltage.js, Group A, 1 New Tile)
> **Status: PROPOSED (2026-08-08). Single-tile spec.** ... Inherits spec.md through spec-v1249.md.
> **The gap.** <why>
## 1. Inheritance and conventions   (dims lint, bounds, worked-example, {error} contract, citation discipline, GOVERNANCE.x, "No table is reproduced")
## 2. The tile  ### 2.1 `id` -- Name   (formula block, Inputs, Outputs)
## 3. Worked example   (hand calculation + cross-checks)
## 4. Scope and non-goals
```
Infrastructure specs such as v1348 use Outcome → Principles table → Scope in/out → UX → … . Commits and CHANGELOG entries cite the spec number.

---

## 9. Other reusable patterns

- **Service worker (`sw.js`).** The shell is an atomic, precached snapshot keyed by `BUILD_HASH`, which the build patches in. It is cache-first, with `skipWaiting`/`clients.claim` and deletion of old caches. The comment explicitly says not to use stale-while-revalidate, because independent background fetches can pair a new `index.html` with a stale `app.js`. Data is cached on first fetch in a separate cache keyed to the same hash. Precaching uses `cache: "reload"`. When offline, a navigation falls back to the cached root document, which then hash-routes. `/api/*` and third-party requests are not intercepted. `check-sw-precache` keeps `SHELL_ASSETS` in sync.
- **`_headers`.**
  - Strict CSP (`default-src 'self'`, one inline script pinned by hash, Turnstile allowed only in `script-src`/`frame-src`, `frame-ancestors 'none'`), HSTS preload, COOP/COEP/CORP, a locked-down Permissions-Policy, and `Referrer-Policy: no-referrer`.
  - Caching: `*.js` and `styles.css` use `max-age=0, must-revalidate`, because file names are stable and a long edge cache would pair new HTML with old JS. `sw.js` is never cached. One `/*.json` rule, because Cloudflare concatenates headers from every matching rule and the last match wins.
- **CHANGELOG** is written by hand as long narrative "Unreleased / Added" entries. I found **no changelog automation**; the only automation nearby is `check-readme-counts` and the generated `llms.txt` counts. Commit messages follow the same style: `fix(scope): <plain-English finding>` plus a narrative body.
- **AGENTS.md conventions:**
  - spec-first work; one new tile touches `tools-data.js`, the compute module, `tool-modules.js declare()`, `compute-map.js` and `worked-examples.json`;
  - "three doors" (web, MCP, report);
  - US standards only; no reproducing copyrighted tables (cite the locator and take table values as inputs);
  - verify against the primary source, not a sibling tile;
  - no hosted runtime except the bounded feedback Worker;
  - always work in a git worktree.

  CLAUDE.md is an OpenLore `orient()` block. The docs set also includes `docs/threat-model.md`, `contributor-checklist.md` and `maintainer-quickstart.md`.
- **Self-honesty gates.** Much of the repo exists to catch docs or tests that claim more than the code does: `check-ci-claims`, spa-head-parity, the 2xx guard in responsive-stress, the three-layer limit comparison in `check-feedback-loop`, and the Phase H disclosure.

---

**Not found:**
- Any OG image generation.
- A triage dashboard or CLI script. Triage is raw `wrangler d1 execute` SQL.
- Any link from a D1 report to a GitHub issue or changelog entry, other than a free-text `resolution_note`.
- A site version or build hash in the report payload.
- Changelog automation.
- A live Lighthouse gate.
- The zone WAF rate-limit rule as code; it is documented only.
