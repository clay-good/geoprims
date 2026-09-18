## Context

Greenfield repository. Motivation is in `proposal.md`. Research that informs these decisions is in `docs/research/01`–`04` (gathered 2026-09-18). Constraints that shape everything:

- **No server compute, ever.** The site is static. All calculation runs on the user's device. The single server component is the opt-in problem-report endpoint, which stores reports and computes nothing.
- **Two surfaces, one math.** The static website and the local MCP server must return byte-identical results. There is deliberately no CLI or public library package: the MCP server's `run` and `pipeline` tools cover scripted use, and the website covers CSV batch work. Fewer surfaces means less to version, document, and support.
- **Hostile numeric environment.** JavaScript engines do not agree on `Math.sin` (V8 and SpiderMonkey use fdlibm ports, JavaScriptCore uses the system libm). Relaxed-SIMD and compiler FMA contraction also vary by CPU.
- **Large reference data.** Geoid grids reach 470 MB (EGM2008 1′), terrain is global, and the CRS registry is about 10 MB as PROJ ships it.
- **Time-varying truth.** Magnetic models expire (WMM2025 valid 2025.0–2030.0). US datums are mid-modernization (NATRF2022, NAPGD2022, GEOID2022, and SPCS2022 are still beta, with adoption expected late 2026 to early 2027). WGS 84 is on realization G2296 (since January 2024).

## Goals / Non-Goals

**Goals:**
- A single Rust source tree compiled to WebAssembly that both surfaces load.
- A manifest format rich enough to generate every UI form, MCP schema, doc page, and search entry.
- Verification infrastructure that makes "matches GeographicLib to 15 nm" a CI fact, not a marketing claim.
- A data-asset pipeline that respects privacy (coarse tiles, same origin) and licenses.

**Non-Goals:**
- Shipping the full PROJ library to browsers by default (see D4).
- Supporting browsers without WebAssembly (the docs still render; tools do not).
- Real-time data ingestion of any kind.

## Decisions

### D1. Rust → WebAssembly for the compute core
Rust compiled to `wasm32-unknown-unknown`, built with `cargo` + `wasm-opt` (binaryen, pinned in `tools/toolchain.json`). wasm-pack was sunset in July 2025, so the pipeline calls the tools directly.

Modules expose a **raw C ABI** rather than wasm-bindgen glue (decided while building task 1.3): `gp_alloc`/`gp_free` for host-owned input buffers, string-returning exports that return a pointer to a core-owned output buffer whose length is read with `gp_out_len()`, plus `gp_version`, and later `gp_invoke`, `gp_invoke_batch`, and `gp_manifest`. Every call crosses the boundary as UTF-8 JSON, so no generated glue is needed. The import section is empty, which makes the import lint trivial, and one small loader in `packages/runtime` serves the browser and Node alike. wasm-bindgen was rejected because its generated JS differs by target (web vs Node), adds a pinned CLI whose version must match the crate, and would put generated code in the zero-dependency MCP server.

| Alternative | Why not |
|---|---|
| Pure TypeScript | Host `Math` differs across engines, which breaks determinism. Slower for iterative geodesy and polyfill. |
| Emscripten C++ (GeographicLib, PROJ, H3 C) | Mature math, but heavy glue, larger binaries, and three C/C++ codebases to build. Still used **in CI** as the differential reference (D9). |
| AssemblyScript | Thin geodesy ecosystem; would mean porting everything. |
| WASI component model | WASI 0.3 was ratified June 2026, but no mainstream MCP host loads components. Revisit after WASI 1.0. |

Key crates (MIT/Apache/BSD only): `geographiclib-rs` (Karney geodesics), `geo` (planar and geodesic algorithms), `h3o` (pure-Rust H3), `s2` (with gaps filled from Go `golang/geo`, Apache-2.0), `robust` (Shewchuk predicates), `i_overlay` or `clipper2` (buffering and boolean ops), `rstar`, `libm`.

### D2. Determinism by construction
- All transcendental math goes through the `libm` crate compiled into the module. Imports of JS `Math` are forbidden by a lint that inspects the Wasm import section.
- Build with a pinned Rust toolchain and the target's default feature set (bulk memory, sign extension, nontrapping float-to-int, multivalue, reference types). SIMD (`+simd128`) may be enabled per module later if a benchmark justifies it; relaxed-SIMD never. FMA contraction is never implicit in Wasm; explicit `mul_add` uses the software path.
- Canonicalize NaN (reject it) and negative zero at the serialization boundary.
- Serialize numbers with the ECMAScript Number-to-String algorithm (the format of `JSON.stringify` and RFC 8785 JCS): shortest round-trip digits, integers without a trailing `.0`, exponent notation only at magnitude ≥ 1e21 or < 1e-6. Implemented in Rust so the core, not the host, produces the bytes.
- The cross-host suite (verification spec) compares bytes across Chromium, Firefox, WebKit, and Node.

### D3. Module split and loading
One Cargo workspace, one crate per domain, compiled into **separate Wasm modules** that share no memory. Shared helpers (units, angles, ellipsoid constants, error model, JSON I/O) live in a `gp-base` crate statically linked into each. The duplicated bytes (~40 KB) cost less than dynamic linking, which Wasm tooling does not support well.

Cross-domain dependencies (for example navigation, survey, drone, and indexing reuse geodesy parsing, frames, and heights; drone reuses the aviation atmosphere) are **statically linked** into each dependent module and count toward that module's 400 KB budget. Endpoints that compose operations from different modules (for example `mgrs-distance`) are composed by the host runtime, which loads each module and chains the calls. The `units` domain (`gp-units`) is linked into `base`, so unit tools run from the base module.

Alternative: one monolithic module (~2–3 MB). Rejected: it breaks the per-page budget in `compute-core`.

### D4. Projections implemented natively; PROJ as oracle, not runtime
Implement the projection methods the catalog needs directly in Rust, following EPSG Guidance Note 7-2 and Karney's Transverse Mercator (6th-order Krüger series, under 5 nm within 3,900 km of the central meridian):

- Transverse Mercator (+ exact TM for far-from-meridian cases)
- Lambert Conformal Conic 1SP/2SP
- Polar Stereographic (UPS)
- Hotine Oblique Mercator (SPCS Alaska 5001, Michigan)
- Albers equal-area
- Web Mercator
- Equidistant cylindrical, Azimuthal equidistant, Orthographic, Gnomonic

The CRS registry (`crs-registry` asset) is a curated EPSG-derived subset: every CRS the catalog uses, with parameters and area-of-use bounds.

| Alternative | Why not |
|---|---|
| proj4js | No epoch-dependent transforms, no automatic choice of transformation, known divergence from PROJ. |
| Full PROJ in Wasm (`wasm-proj`) | Several MB of Wasm plus a ~10 MB `proj.db`, synchronous network grid fetches, and no npm package. Kept as a possible later "advanced CRS lab" behind an explicit download. |

PROJ 9.x runs in CI as the differential oracle for every projection and datum transformation.

### D5. Manifest-first tool definition
Each tool is a Rust function with a declarative definition (inputs, outputs, units, ranges, errors, references, vectors, visualization): a `static ToolDef` built from plain const data, with small `macro_rules!` helpers only where a family repeats (for example `convert_op!` and `pair!` in the units domain). Plain data needs no procedural macro, and the compiler checks every field. Each module's `gp_manifest()` export serializes its definitions, so the catalog is built from the exact bytes that ship. The build emits:

1. `catalog/v1.json` (all manifests, JSON Schema 2020-12 with `x-` extensions).
2. TypeScript types for the web app and the MCP server.
3. MCP tool schemas and descriptions.
4. Search-index documents.
5. Docs page scaffolds (humans add the prose: formula explanation, worked example).

Alternative: hand-written JSON manifests beside Rust code. Rejected because they drift from the code.

### D6. Repository layout
```
core/                 Rust workspace (gp-base, gp-geodesy, gp-navigation, gp-geometry,
                      gp-aviation, gp-drone, gp-survey, gp-indexing, gp-raster, gp-time, gp-units)
core/vectors/         Golden vectors (JSON Lines, one file per tool), immutable history
tools/codegen/        Manifest → schema/TS/MCP/search/docs generators
assets/               Asset registry, build scripts, tilers (outputs not committed)
packages/runtime/     Internal (unpublished) Wasm loader, validation, asset providers
                      (browser, Node); shared by the website and the MCP server
mcp/                  Local MCP server: zero-dependency server.mjs; release tags carry
                      prebuilt dist/ so a clone runs with Node alone (also published to npm)
apps/web/             Static site + PWA
worker/               The single Cloudflare Worker: static-asset config, /api/reports*
                      handler, D1 migrations, cron (per define-build-contracts B1)
verify/               Differential harness (GeographicLib C++, PROJ, H3 C, S2 C++, WMM C)
openspec/             Specs and changes
docs/research/        Research briefs backing the specs
```

### D7. Hosting: Cloudflare static hosting plus an asset bucket
- Site on Cloudflare Workers static assets (Cloudflare's recommended path for new projects) for custom headers (CSP, COOP/COEP where needed). Static requests never invoke code.
- The only server code is the problem-report Worker on `/api/reports*` with its D1 database (per `add-problem-reporting`). It performs no calculation.
- Large tiled assets on an R2 bucket behind `assets.geoprims.com`, with CORS for the site origin and `Range` support.
- GitHub Pages was rejected: it cannot set CSP or COOP/COEP headers.
- The site works without cross-origin isolation (threads are optional, per `compute-core`), so a header regression degrades speed, not correctness.

### D8. Honest counting: operations vs endpoints
Per `tool-catalog`, the public count reports both numbers. The target is about **514 operations** and about **834 tool ids** (restated in `plan-launch-and-value-proof`); the per-domain rollup is below. Endpoint expansion comes from an allow-listed conversion graph (for example `dms-to-utm`, `kt-to-mph`), never blind permutation. This keeps the "800+ tools" claim true without padding (alias slugs such as `crosswind-calculator` are search aliases, not tool ids, and are never counted). Public counts include only stable tools, so the claim is made only once the stable count supports it.

### D9. Verification harness
- Golden vectors live beside each tool as JSON Lines with `source`, `sourceVersion`, and per-field tolerances.
- A differential runner builds reference binaries in a CI container (GeographicLib C++ 2.x, PROJ 9.x, H3 C 4.5, S2 C++, NOAA WMM C, NGS HTDP) and compares 10,000 random and edge-biased inputs per family.
- Property tests use `proptest` in Rust.
- Browser determinism runs through Playwright in Chromium, Firefox, and WebKit.

### D10. Versioning
- The core has a single semver for the whole catalog release (`coreVersion`).
- Each tool carries its own semver (`toolVersion`). The major version bumps when its numeric behavior changes beyond tolerance or its schema breaks.
- Assets are versioned independently by their issuer version (e.g. `wmm2025`) plus a geoprims packaging revision.

### D11. Aviation and operational safety posture
Tools are planning and education aids, not certified navigation. Regulatory values (Part 107 limits, EASA classes, cold-temperature airport lists) are "reference data" with an effective date and a link to the authority. Where a rule is proposed but not final (FAA Part 108 as of September 2026), the tool labels it "Proposed" with the Federal Register citation.

### D12. Export-control posture
All content is published openly without restriction, which places it outside the EAR (15 CFR 734.3(b)(3), 734.7). The catalog excludes weapons-oriented functions (ballistic trajectories, fire control, targeting solutions, munitions guidance) and cryptography. Counsel reviews the catalog before launch (task 9.4).

## Endpoint rollup (targets; details in each domain change's design)

Restated by `plan-launch-and-value-proof` (design L6) after the practitioner review.

| Domain | Operations | Tool ids | Defined in |
|---|---|---|---|
| geodesy | 89 | 173 | `add-geodesy-suite` |
| navigation | 46 | 58 | `add-navigation-and-geometry` |
| geometry | 38 | 44 | `add-navigation-and-geometry` |
| aviation | 97 | 117 | `add-aviation-suite`, `add-practitioner-essentials` |
| drone | 49 | 49 | `add-drone-suite`, `add-practitioner-essentials` |
| survey | 72 | 72 | `add-survey-suite`, `add-practitioner-essentials` |
| indexing | 52 | 92 | `add-spatial-indexing-and-raster` |
| raster | 30 | 40 | `add-spatial-indexing-and-raster` |
| time | 19 | 19 | `add-practitioner-essentials` |
| units | 22 | 170 | this change (`units` domain; pair pages such as `kt-to-mph`) |
| **Total** | **514** | **834** | |

Tool ids are not all indexable pages. About 300 tool pages are indexable. Other generated ids resolve as presets canonicalized to their parent tool (per `discovery/search-pages`).

## Risks / Trade-offs

- **[`geographiclib-rs` does not cover everything GeographicLib C++ does (Rhumb, exact TM, GeodesicExact)]** → Port the missing classes from C++ (MIT). Differential tests against C++ guard the port.
- **[h3o parity with H3 4.5 (especially polygon containment modes) is unconfirmed]** → Conformance suite against the H3 C library for every API and resolution. If a gap exists, compile H3 C to Wasm for that function only.
- **[The Rust `s2` crate is incomplete]** → Port the needed pieces (cell ID math, RegionCoverer) from Go `golang/geo`. Test against S2 C++.
- **[Determinism break from a toolchain upgrade]** → Pinned toolchain. Upgrades go through a PR that must pass the cross-host byte-comparison suite.
- **[Asset hosting cost (terrain egress)]** → Coarse tiles, long-lived immutable caching, offline packs, and R2 (no egress fees).
- **[Datum modernization lands mid-build (NATRF2022 adoption)]** → Datum data is versioned in the registry. The beta label flips via a registry update, not a code change.
- **[Hundreds of tools dilute quality]** → Tools ship as `experimental` until they meet the stable bar. Public counts include only stable tools.
- **[Manifest DSL becomes a bottleneck]** → Keep it declarative and small. Escape hatches are allowed for custom visualization only.

## Migration Plan

Not applicable (greenfield). Release process: tag → reproducible CI build → verification report → deploy static site and assets → publish the MCP server to npm with provenance → publish MCPB bundle and registry entry.

## Open Questions

- Exact SPCS2022 zone count and final EPSG codes: fetch from NGS when the zone-definition page is available. Data only; no spec change.
- GEOID2022 grid sizes and tiling: measure when NGS publishes final files.
- Whether to offer an optional full-PROJ "CRS lab" later: decide after launch based on demand. Not in any current task.
