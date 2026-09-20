## 1. Repository and toolchain

- [x] 1.1 Create the monorepo layout from design D6 (core/, tools/codegen/, assets/, packages/, apps/web/, verify/) and verify `ls` shows each directory with a README stating its purpose
- [x] 1.2 Pin the Rust toolchain (`rust-toolchain.toml`) and wasm-opt version (`tools/toolchain.json`; no wasm-bindgen per design D1); verify CI prints the pinned versions and fails if they differ
- [x] 1.3 Create the Cargo workspace with `gp-base` and empty domain crates; verify `cargo build --target wasm32-unknown-unknown` produces one `.wasm` per domain
- [x] 1.4 Add the Wasm import-section lint that fails if any module imports JS `Math` or anything outside the allow-list; verify with a deliberately bad fixture crate
- [x] 1.5 Add compressed size budgets per module (base ≤ 120 KB, domain ≤ 400 KB Brotli); verify CI fails on an oversized fixture

## 2. Base crate: units, angles, errors, serialization

- [x] 2.1 Implement the unit registry with the exact constants table and aliases; verify unit tests for every row of the constants table and the NM, survey-foot, and inHg scenarios
- [x] 2.2 Implement unit-tagged value parsing (aliases, case rules, separators, decimal-comma mode, bare `mil` rejection, `nm` assumption warning); verify table-driven parser tests including every scenario in the units spec
- [x] 2.3 Implement temperature vs temperature-difference quantities; verify the +18 °F → +10 K scenario
- [x] 2.4 Implement angle normalization (longitude `[-180,180)`, azimuth `[0,360)`, latitude validation) using exact remainders; verify the 180, 540.25, and 90.0000001 scenarios and a 1,000,000-case property test
- [x] 2.5 Implement the structured error model and warning codes; verify every code serializes with `code`, `message`, `field`, `hint`
- [x] 2.6 Implement result serialization (shortest round-trip, negative-zero normalization, NaN rejection, `meta` provenance block); verify byte-exact snapshot tests
- [x] 2.7 Implement output unit profiles (`si`, `aviation`, `us-customary`, `survey-metric`, `survey-us`); verify the aviation-profile scenario

## 3. Manifest DSL and code generation

- [x] 3.1 Define the manifest meta-schema (JSON Schema 2020-12 plus every extension listed in `contracts/manifest-extensions`, closed to unknown `x-` fields); verify it validates a hand-written sample manifest and rejects each missing required field (the core's lint and tools/trust/metaschema.mjs together are the meta-schema: the closed extension set, each extension's shape, and every structural rule. The hand-written sample in core/crates/gp-base/tests/fixtures/sample-manifest.json is pinned byte for byte, and a sweep blanks each of its eleven required fields in turn and fails if any blanked manifest is accepted)
- [x] 3.2 Implement the Rust tool definition (a static `ToolDef` per tool, per design D5) that emits manifests; verify a sample tool produces a manifest identical to a checked-in snapshot
- [x] 3.3 Implement the id-pattern, alias-uniqueness, inverse-symmetry, visualization-mapping, and reference checks; verify each fails on a targeted bad fixture
- [x] 3.4 Generate `catalog/v1.json`, TypeScript types, MCP schemas, and search documents from manifests; verify a snapshot test over the sample tool for each output (the pinned sample snapshots and full-catalog schema and search-parity checks pass)
- [x] 3.5 Implement the conversion-graph generator with pair allow-list and `composedOf`; verify that a non-allow-listed pair produces no endpoint
- [x] 3.6 Emit operation and endpoint counts from the build; verify the counts match the manifests

## 4. Compute core runtime

- [x] 4.1 Implement the uniform `invoke`, `invokeBatch`, `manifest`, `version` exports per module; verify the unknown-tool and batch-order scenarios
- [x] 4.2 Implement the host asset-provider interface and `ASSET_UNAVAILABLE` flow; verify with a mock provider that supplies, withholds, and corrupts an asset
- [x] 4.3 Implement per-invocation memory caps and `LIMIT_EXCEEDED` pre-checks; verify with an over-limit fixture that no allocation beyond the cap occurs (packages/runtime/src/limits.test.mjs drives every capped list input from the tool's own example rows: one row over the cap is refused with LIMIT_EXCEEDED naming the cap and returning no result, and the cap itself still works, so it is not off by one. The byte cap is checked to refuse a payload before it is parsed)
- [ ] 4.4 Implement progress reporting and cooperative cancellation for long tools; verify cancellation returns within 100 ms in a benchmark fixture (built so far: the web client interrupts a superseded Wasm call by replacing its worker, discards its answer, replays unrelated pending calls, and shows elapsed time every 250 ms; both a simulated stuck-call benchmark and a real H3 polygon calculation in Chromium stop within 100 ms of an edit. The Node worker host accepts an AbortSignal, reports elapsed time every 250 ms, and passes a real spinning-worker cancellation benchmark within 100 ms. The MCP stdio server forwards opt-in progress notifications and honors cancellation for running and queued calls. Pending: core-level cooperative cancellation)
- [x] 4.5 Implement the browser worker host and trap containment; verify a trapping fixture restarts the worker and other tools keep working (the module-level containment has a trapping fixture in packages/runtime; the browser worker on top of it is now driven the way the page drives it, in apps/web/test/worker.test.mjs: a real call, an unknown tool, and a bad input all come back as envelopes, the worker keeps serving after each, the search path works through the same worker, and every message gets exactly one reply, so nothing can leave the answer card marked stale forever)
- [x] 4.6 Implement the internal `packages/runtime` loader with browser and Node hosts (same Wasm, same-origin cache and filesystem asset providers); verify the cross-surface digest scenario (the website and MCP artifact gates check every built Wasm SHA-256 digest, and the browser worker and Node host return byte-identical envelopes for every live golden vector, including asset-backed tools)

## 5. Data assets

- [ ] 5.1 Implement the asset registry format and validator; verify every initial-dataset row is present with license, attribution, digest, and load policy (built: assets/registry.json with egm96-15, wmm2025, igrf14, and nadcon5, validated for fields, sizes, and digests, and now cross-checked against the catalog in both directions: a dataset a tool uses must be registered, and a registered dataset no tool uses is caught as dead weight the licenses page would otherwise show. Each row's source URL, retrieval date, load policy, and attribution are checked for shape. Pending: the remaining initial datasets)
- [ ] 5.2 Build asset pipelines for WMM2025, WMMHR2025, and IGRF-14 (coefficient files, public domain); verify digests and the NCEI test values load
- [ ] 5.3 Build geoid tilers for EGM96-15, EGM2008-2.5, EGM2008-1, and GEOID18 (tiles ≥ 1° × 1°, per-tile digests, signed index); verify tile-size floor and digest checks
- [ ] 5.4 Build NADCON5 grid packaging from NGS sources; verify against PROJ-data digests and NGS sample points
- [ ] 5.5 Build the curated `crs-registry` from EPSG plus NGS SPCS2022-beta definitions, with bounds and status; verify every CRS referenced by any tool is present
- [ ] 5.6 Build the Copernicus GLO-30 re-tiler to self-hosted tiles with the exact required attribution; verify tiles open and attribution text matches the license
- [ ] 5.7 Package Natural Earth 110m and 50m; verify sizes within budget
- [x] 5.8 Implement integrity verification and eviction in both hosts; verify the corrupted-tile scenario (the shared provider checks registry SHA-256 before either host supplies bytes; Node reads its packaged files without a cache and refuses corrupt bytes. The browser evicts a corrupt asset from app and offline-pack caches, retries from the network, and caches only the verified replacement. A real service-worker test proves the first call returns ASSET_INTEGRITY, the retry succeeds, and a fresh worker can use the replacement offline)
- [ ] 5.9 Implement validity-window enforcement and the `MODEL_EXTRAPOLATED` / `OUT_OF_DOMAIN` behaviors; verify the WMM 2030.5 scenario
- [ ] 5.10 Implement the `NON_OFFICIAL_DATUM` labeling from registry status; verify the SPCS2022 beta scenario
- [x] 5.11 Write the exclusions list (what3words, EMM) with reasons; verify it renders on `/licenses` (what3words, the Enhanced Magnetic Model, copyrighted design tables, and commercial basemap tiles, each with the reason, rendered on /licenses/ and checked by a test)

## 6. Verification infrastructure

- [x] 6.1 Define the golden-vector file format (JSON Lines, source, version, tolerances, supersession); verify the lint rejects missing provenance and silent edits
- [ ] 6.2 Build the differential-reference container (GeographicLib C++, PROJ 9.x, H3 C 4.5, S2 C++, WMM C, HTDP); verify each reference binary runs a smoke case in CI
- [ ] 6.3 Implement the differential runner (10,000 random and edge-biased cases per family, tolerance comparison, failing-case report); verify with a deliberately perturbed tool that it fails
- [x] 6.4 Implement the cross-host determinism suite (Playwright Chromium/Firefox/WebKit plus Node, byte comparison); verify it passes on the sample tool and fails on an injected host-Math fixture (the browser CI gate compares the serialized result of every live golden vector across all four hosts; 2,705 vectors passed locally. The import-lint fixture rejects a module that imports `Math.sin` before it can reach any host)
- [ ] 6.5 Implement the benchmark harness (reference profile, p50/p95, diff against previous release); verify output table and regression failure on a slowed fixture (built so far: Node and Chromium main-thread runners each measure all 205 primary examples with 50 warm-up and 1,000 timed calls per tool, emit p50/p95 tables and JSON reports, compare same-profile reports, and fail a slowed fixture above a 20% p95 regression. Chromium uses the 4× CPU setting, gates cold module instantiation at 150 ms, and uploads its report in CI. Pending: per-tool execution budgets and a previous-release baseline)
- [x] 6.6 Implement verification report generation (`/verification/<version>`); verify it lists vector counts, sources, max error, and tolerances (tools/trust/verification.mjs reruns every live vector through the built modules; the current report records 2,705 vectors, 10,242 checks, and zero failures. The page and a JSON download list each tool's worst check with its tolerance and share used)

## 7. Privacy and security

- [ ] 7.1 Write the CSP and security header configuration for the static host; verify the header smoke test on a preview deployment
- [x] 7.2 Implement the egress privacy test (sentinel inputs, proxy capture, zero sentinel matches); verify it catches a deliberately leaking fixture (apps/web/test/egress.test.mjs checks bundled network primitives and report payloads, including private-field withholding. The Chromium end-to-end test now types a unique sentinel on all 205 tool pages, captures request URLs, headers, and bodies including compute-worker Wasm fetches, and finds no sentinel or third-party request. Its injected leaking fetch proves the proxy catches a bad request)
- [x] 7.3 Implement the third-party request audit over all routes; verify it fails on an injected external font (apps/web/test/security.test.mjs scans every built page and stylesheet, catching injected Google Fonts, a CDN font, and a tracking pixel. Playwright captures live requests across all 292 built routes, observes zero third-party requests before the report dialog opens, and permits exactly the Turnstile script when reporting is enabled)
- [x] 7.4 Set up reproducible builds and publish SHA-256 digests per artifact; verify two CI runs produce identical digests (the local `verify:reproducible` command compares two clean Wasm builds, and the integrity gates check recorded digests against website and MCP copies. CI now builds core, MCP, and website artifacts on two independent runners with the build date pinned to the commit timestamp, then compares SHA-256 manifests for every shipped file. Two local full builds matched across 818 files; the independent-runner gate still needs a successful CI run)
- [ ] 7.5 Configure npm trusted publishing (OIDC) with provenance and dependency license/vulnerability audits; verify a dry-run publish shows provenance
- [x] 7.6 Add `SECURITY.md` and `/.well-known/security.txt`; verify RFC 9116 validity (SECURITY.md says where to report privately, what is in scope per component, what is not, and that a wrong answer goes to the report button instead; /.well-known/security.txt carries Contact, Expires, Preferred-Languages, Canonical, and Policy, and a test fails 30 days before it expires, or if it is dated more than a year out)

## 8. Catalog and lifecycle

- [x] 8.1 Implement the domain/group taxonomy file and validation; verify every id maps to exactly one group (data/taxonomy.json names all ten domains and their groups, including the planned raster groups. Each crate checks its own tool ids in Rust; catalog generation now checks the full 205-tool set for missing or duplicate groups, duplicate ids across modules, and disagreement between each id and its declared domain/group. Targeted bad fixtures fail, and the built catalog passes)
- [x] 8.2 Implement lifecycle states and the stable-promotion gate (≥ 20 vectors, differential tests passing); verify promotion fails for a tool with 19 vectors (the gate runs over every stable tool in every test run; the spec's scenario is now pinned directly: a passing tool with its vector count set to 19 is refused with exactly that problem, 20 is accepted, and a tool still advertising the EXPERIMENTAL_TOOL warning cannot be stable)
- [x] 8.3 Implement deprecation redirects and deprecation notices in results; verify the deprecated-id scenario (a deprecated tool still runs and its envelope carries meta.deprecation with the replacement id and the removal version; its page canonicalizes to the replacement and leaves the index; and the route gate writes its old route into dist/_redirects from the day it is deprecated, so the link survives removal. No tool is deprecated yet, so both halves are checked on fixtures: an envelope test in the core, and a route test that also holds a plain rename to the stricter rule that it may not shadow a live page)
- [x] 8.4 Implement the `units` domain operations and allow-listed pair endpoints; verify the kt-to-mph and fuel-density scenarios

## 9. Release readiness

- [ ] 9.1 Write the release pipeline (tag → build → verify → deploy site and assets → publish MCP server); verify on a release-candidate tag in a staging environment
- [ ] 9.2 Publish the `/licenses` page generated from the registry and SBOM; verify every dependency and dataset appears (done: /licenses/ renders every asset-registry row with its licence and required attribution, the fonts under the OFL, the Natural Earth base map with the attribution its licence asks for, the build dependencies from the website's own package.json, and the exclusions list. A test fails if a registry row is missing. Pending: a full SBOM, which waits on the release pipeline)
- [x] 9.3 Write the site-wide disclaimer, privacy page, and accuracy policy; verify each is linked from every page footer (/accuracy/ explains status, tolerances, observed error, assumptions, and corrections; the web build and trust-page test verify all three links on every generated HTML page)
- [ ] 9.4 Obtain export-control and liability review of the catalog and disclaimers; verify written sign-off is recorded before public launch
