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

- [ ] 3.1 Define the manifest meta-schema (JSON Schema 2020-12 plus every extension listed in `contracts/manifest-extensions`, closed to unknown `x-` fields); verify it validates a hand-written sample manifest and rejects each missing required field
- [x] 3.2 Implement the Rust tool definition (a static `ToolDef` per tool, per design D5) that emits manifests; verify a sample tool produces a manifest identical to a checked-in snapshot
- [x] 3.3 Implement the id-pattern, alias-uniqueness, inverse-symmetry, visualization-mapping, and reference checks; verify each fails on a targeted bad fixture
- [ ] 3.4 Generate `catalog/v1.json` (done: `tools/codegen/catalog.mjs`), TypeScript types, MCP schemas, and search documents from manifests; verify a snapshot test over the sample tool for each output
- [x] 3.5 Implement the conversion-graph generator with pair allow-list and `composedOf`; verify that a non-allow-listed pair produces no endpoint
- [x] 3.6 Emit operation and endpoint counts from the build; verify the counts match the manifests

## 4. Compute core runtime

- [x] 4.1 Implement the uniform `invoke`, `invokeBatch`, `manifest`, `version` exports per module; verify the unknown-tool and batch-order scenarios
- [x] 4.2 Implement the host asset-provider interface and `ASSET_UNAVAILABLE` flow; verify with a mock provider that supplies, withholds, and corrupts an asset
- [x] 4.3 Implement per-invocation memory caps and `LIMIT_EXCEEDED` pre-checks; verify with an over-limit fixture that no allocation beyond the cap occurs (packages/runtime/src/limits.test.mjs drives every capped list input from the tool's own example rows: one row over the cap is refused with LIMIT_EXCEEDED naming the cap and returning no result, and the cap itself still works, so it is not off by one. The byte cap is checked to refuse a payload before it is parsed)
- [ ] 4.4 Implement progress reporting and cooperative cancellation for long tools; verify cancellation returns within 100 ms in a benchmark fixture
- [ ] 4.5 Implement the browser worker host and trap containment; verify a trapping fixture restarts the worker and other tools keep working
- [ ] 4.6 Implement the internal `packages/runtime` loader (done: shared module loader, input hardening, Node host) with browser and Node hosts (same Wasm, cache and filesystem asset providers); verify the cross-surface digest scenario (asset providers built: filesystem for Node and MCP, same-origin fetch in the browser worker, both checking SHA-256 with WebCrypto)

## 5. Data assets

- [ ] 5.1 Implement the asset registry format and validator; verify every initial-dataset row is present with license, attribution, digest, and load policy (built: assets/registry.json with egm96-15, wmm2025, igrf14, and nadcon5, validated for fields, sizes, and digests, and now cross-checked against the catalog in both directions: a dataset a tool uses must be registered, and a registered dataset no tool uses is caught as dead weight the licenses page would otherwise show. Each row's source URL, retrieval date, load policy, and attribution are checked for shape. Pending: the remaining initial datasets)
- [ ] 5.2 Build asset pipelines for WMM2025, WMMHR2025, and IGRF-14 (coefficient files, public domain); verify digests and the NCEI test values load
- [ ] 5.3 Build geoid tilers for EGM96-15, EGM2008-2.5, EGM2008-1, and GEOID18 (tiles ≥ 1° × 1°, per-tile digests, signed index); verify tile-size floor and digest checks
- [ ] 5.4 Build NADCON5 grid packaging from NGS sources; verify against PROJ-data digests and NGS sample points
- [ ] 5.5 Build the curated `crs-registry` from EPSG plus NGS SPCS2022-beta definitions, with bounds and status; verify every CRS referenced by any tool is present
- [ ] 5.6 Build the Copernicus GLO-30 re-tiler to self-hosted tiles with the exact required attribution; verify tiles open and attribution text matches the license
- [ ] 5.7 Package Natural Earth 110m and 50m; verify sizes within budget
- [ ] 5.8 Implement integrity verification and eviction in both hosts; verify the corrupted-tile scenario
- [ ] 5.9 Implement validity-window enforcement and the `MODEL_EXTRAPOLATED` / `OUT_OF_DOMAIN` behaviors; verify the WMM 2030.5 scenario
- [ ] 5.10 Implement the `NON_OFFICIAL_DATUM` labeling from registry status; verify the SPCS2022 beta scenario
- [x] 5.11 Write the exclusions list (what3words, EMM) with reasons; verify it renders on `/licenses` (what3words, the Enhanced Magnetic Model, copyrighted design tables, and commercial basemap tiles, each with the reason, rendered on /licenses/ and checked by a test)

## 6. Verification infrastructure

- [x] 6.1 Define the golden-vector file format (JSON Lines, source, version, tolerances, supersession); verify the lint rejects missing provenance and silent edits
- [ ] 6.2 Build the differential-reference container (GeographicLib C++, PROJ 9.x, H3 C 4.5, S2 C++, WMM C, HTDP); verify each reference binary runs a smoke case in CI
- [ ] 6.3 Implement the differential runner (10,000 random and edge-biased cases per family, tolerance comparison, failing-case report); verify with a deliberately perturbed tool that it fails
- [ ] 6.4 Implement the cross-host determinism suite (Playwright Chromium/Firefox/WebKit plus Node, byte comparison); verify it passes on the sample tool and fails on an injected host-Math fixture
- [ ] 6.5 Implement the benchmark harness (reference profile, p50/p95, diff against previous release); verify output table and regression failure on a slowed fixture
- [x] 6.6 Implement verification report generation (`/verification/<version>`); verify it lists vector counts, sources, max error, and tolerances (tools/trust/verification.mjs reruns every live vector through the built modules, 1,014 vectors and 3,262 checks in about 150 ms; the page and a JSON download list each tool's worst check with its tolerance and share used)

## 7. Privacy and security

- [ ] 7.1 Write the CSP and security header configuration for the static host; verify the header smoke test on a preview deployment
- [ ] 7.2 Implement the egress privacy test (sentinel inputs, proxy capture, zero sentinel matches); verify it catches a deliberately leaking fixture
- [x] 7.3 Implement the third-party request audit over all routes; verify it fails on an injected external font (apps/web/test/security.test.mjs walks every built page and stylesheet for anything the browser would fetch from another origin: src, href, data, srcset, poster, CSS url() and @import. Links and the canonical are not requests, and only challenges.cloudflare.com and assets.geoprims.com are allowed. Injected Google Fonts, a CDN @font-face, and a tracking pixel are each caught)
- [x] 7.4 Set up reproducible builds and publish SHA-256 digests per artifact; verify two CI runs produce identical digests (the build is reproducible today: `npm run verify:reproducible` builds all twelve modules twice from a cleared target directory and compares the digests, and they are identical. The digests are published on /verification/<version>/ with the command to check them, and tools/wasm/integrity.test.mjs runs in every test run: it fails if a recorded digest does not match the module on disk, if a module is built but unrecorded or recorded but unbuilt, or if the MCP release ships bytes that are not the built ones; the website's half of the same check lives in the site's own suite, because public/wasm only exists once the site is built, and CI now builds mcp/dist before the tests so neither half silently skips. Pending: running the two-build comparison in CI, which needs CI)
- [ ] 7.5 Configure npm trusted publishing (OIDC) with provenance and dependency license/vulnerability audits; verify a dry-run publish shows provenance
- [x] 7.6 Add `SECURITY.md` and `/.well-known/security.txt`; verify RFC 9116 validity (SECURITY.md says where to report privately, what is in scope per component, what is not, and that a wrong answer goes to the report button instead; /.well-known/security.txt carries Contact, Expires, Preferred-Languages, Canonical, and Policy, and a test fails 30 days before it expires, or if it is dated more than a year out)

## 8. Catalog and lifecycle

- [ ] 8.1 Implement the domain/group taxonomy file and validation; verify every id maps to exactly one group
- [x] 8.2 Implement lifecycle states and the stable-promotion gate (≥ 20 vectors, differential tests passing); verify promotion fails for a tool with 19 vectors (the gate runs over every stable tool in every test run; the spec's scenario is now pinned directly: a passing tool with its vector count set to 19 is refused with exactly that problem, 20 is accepted, and a tool still advertising the EXPERIMENTAL_TOOL warning cannot be stable)
- [x] 8.3 Implement deprecation redirects and deprecation notices in results; verify the deprecated-id scenario (a deprecated tool still runs and its envelope carries meta.deprecation with the replacement id and the removal version; its page canonicalizes to the replacement and leaves the index; and the route gate writes its old route into dist/_redirects from the day it is deprecated, so the link survives removal. No tool is deprecated yet, so both halves are checked on fixtures: an envelope test in the core, and a route test that also holds a plain rename to the stricter rule that it may not shadow a live page)
- [x] 8.4 Implement the `units` domain operations and allow-listed pair endpoints; verify the kt-to-mph and fuel-density scenarios

## 9. Release readiness

- [ ] 9.1 Write the release pipeline (tag → build → verify → deploy site and assets → publish MCP server); verify on a release-candidate tag in a staging environment
- [ ] 9.2 Publish the `/licenses` page generated from the registry and SBOM; verify every dependency and dataset appears (done: /licenses/ renders every asset-registry row with its licence and required attribution, the fonts under the OFL, the Natural Earth base map with the attribution its licence asks for, the build dependencies from the website's own package.json, and the exclusions list. A test fails if a registry row is missing. Pending: a full SBOM, which waits on the release pipeline)
- [ ] 9.3 Write the site-wide disclaimer, privacy page, and accuracy policy; verify each is linked from every page footer (done: /disclaimer/ and a new /privacy/ that states what runs on the device, what the browser keeps, the one thing that is ever sent and exactly what it carries, the bot check as the only third-party code, and that no IP address is stored; the footer links both on every page, and tests check the promises and the links. Pending: the accuracy policy as its own page, which /methodology/ currently carries)
- [ ] 9.4 Obtain export-control and liability review of the catalog and disclaimers; verify written sign-off is recorded before public launch
