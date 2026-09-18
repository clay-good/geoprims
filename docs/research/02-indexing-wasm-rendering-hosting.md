# Research: spatial indexing, WebAssembly toolchain, rendering, and hosting

> Research brief gathered 2026-09-18 to inform the OpenSpec changes. Items marked [mem] are from memory and still need checking against a primary source.

The recommended stack is Rust compiled to one Wasm module with wasm-bindgen, drawn with luma.gl on WebGPU (WebGL2 fallback), hosted on Cloudflare. Four constraints shape it:

1. **No live DEM (terrain elevation) from Copernicus in the browser.** Its public S3 bucket sends no CORS header, so you have to mirror or convert the tiles yourself.
2. **Plan to run single-threaded.** Threads need cross-origin isolation (COOP/COEP headers). GitHub Pages can't set those headers, and Safari doesn't support the easier `credentialless` option.
3. **Firefox on Linux and Android doesn't ship WebGPU yet.** Chrome on Linux ships it only for some GPUs, so a WebGL2 fallback is required.
4. **what3words is excluded on legal grounds.** The algorithm is patented and its terms forbid reverse engineering.

Package and crate versions below come from the live npm and crates.io registries on 2026-09-18. Sizes I measured myself are marked. Items marked [mem] come from my own knowledge and still need checking against the source.

## 1. H3
- **Versions:** H3 core v4.5.0 was released 2026-05-21. h3-js 4.5.0 wraps it (released 2026-07-01). Apache-2.0. ([h3-js](https://github.com/uber/h3-js))
- **Build and size:** h3-js is compiled with emscripten to plain asm.js-style JavaScript, not a separate `.wasm` file. I found no "WebAssembly" string in the UMD dist. It measures about 216 KB minified and 65 KB gzipped. Benchmarks in the README: `latLngToCell` about 430K ops/sec, `isValidCell` about 3.65M ops/sec.
- **v4 API names:** `latLngToCell`, `cellToLatLng`, `cellToBoundary`, `gridDisk`, `gridDistance`, `polygonToCells`, `cellsToMultiPolygon`. There is also `polygonToCellsExperimental(polygon, res, flag)`, where `POLYGON_TO_CELLS_FLAGS` is one of `containmentCenter`, `containmentFull`, `containmentOverlapping` or `containmentOverlappingBbox`. Most v3 names changed in v4.
- **h3o:** a from-scratch pure-Rust rewrite, not a binding, with no C dependencies, so it compiles to Wasm cleanly. BSD-3-Clause, v0.11.0 (2026-08-29). ([h3o](https://github.com/HydroniumLabs/h3o), [crates.io](https://crates.io/crates/h3o)) I did not confirm that it has all four containment modes or exact parity with H3 4.5; check before committing.
- **Resolution table** ([h3geo restable](https://h3geo.org/docs/core-library/restable/)). Every resolution has exactly 12 pentagons. Resolution 0 has 122 cells; resolution 15 has 569,707,381,193,162.

| Res | Avg hex area (km²) | Avg edge (km) |
|---|---|---|
| 0 | 4,357,449 | 1,281.26 |
| 1 | 609,788 | 483.06 |
| 2 | 86,802 | 182.51 |
| 3 | 12,393 | 68.98 |
| 4 | 1,770 | 26.07 |
| 5 | 252.9 | 9.85 |
| 6 | 36.13 | 3.72 |
| 7 | 5.16 | 1.41 |
| 8 | 0.74 | 0.53 |
| 9 | 0.11 | 0.20 |
| 10 | 0.015 | 0.076 |
| 11 | 0.002 | 0.029 |
| 12 | 0.0003 | 0.011 |
| 13 | 4.4e-5 | 0.0041 |
| 14 | 6.3e-6 | 0.0015 |
| 15 | 9e-7 | 0.0006 |

## 2. S2, Quadkey, Geohash, other grids
- **S2 in JavaScript:** `s2js` 1.44.0 (Apache-2.0) is a full TypeScript port with roughly the same features as Go's `golang/geo`. It includes a RegionCoverer and GeoJSON helpers. ([s2js.org](https://s2js.org/)) The older npm `s2-geometry` package only converts between lat/lng and keys; use s2js instead.
- **S2 in Rust:** the `s2` crate is v0.2.0 (2026-08-19). It is less complete than Go or C++ [mem].
- **S2 structure** [mem]: levels 0 to 30. A 64-bit cell ID holds 3 bits for the cube face, then the Hilbert-curve position, then a trailing 1 bit. A token is the ID in hex with trailing zeros removed. Level 30 cells average about 0.74 cm²; level 0 is one cube face, about 85M km².
- **Bing Quadkey** [mem]: Web Mercator tiles. Each zoom level adds one base-4 digit (0–3), so key length equals zoom (1–23).
- **Geohash** [mem]: 32-character base32 alphabet `0123456789bcdefghjkmnpqrstuvwxyz` (no a, i, l, o). Cell size by length:

| Length | Cell size |
|---|---|
| 1 | 5,000 × 5,000 km |
| 2 | 1,250 × 625 km |
| 3 | 156 × 156 km |
| 4 | 39.1 × 19.5 km |
| 5 | 4.89 × 4.89 km |
| 6 | 1.22 × 0.61 km |
| 7 | 153 × 153 m |
| 8 | 38.2 × 19.1 m |
| 9 | 4.77 × 4.77 m |
| 10 | 1.19 m × 0.60 m |
| 11 | 149 × 149 mm |
| 12 | 37 × 19 mm |

  For a US audience, convert the geohash sizes to US units in prose.
- **A5 (Felix Palmer):** pentagonal cells tiled on a dodecahedron. Cells at each resolution have equal area. 31 resolutions, the smallest under 30 mm². Apache-2.0. `a5-js` 0.10.1 is written in TypeScript, about 118 KB minified and 29 KB gzipped (measured). API includes `lonLatToCell` and `cellToBoundary`. There are Rust (`a5-rs`) and Python ports, and it ties into deck.gl. Still pre-1.0, so expect API churn. ([a5](https://github.com/felixpalmer/a5))
- **OGC API – DGGS Part 1: Core:** approved as an official OGC standard around October 2025. It is a Web API standard, so for this product it matters only as terminology: DGGRS, zones, zone IDs. ([OGC](https://www.ogc.org/announcement/ogc-membership-approves-ogc-api-discrete-global-grid-systems-part-1-core-as-an-official-ogc-standard/))
- **Plus Codes / Open Location Code:** Apache-2.0, with official JavaScript and Rust implementations. A 10-digit code is about 14 × 14 m and an 11-digit code about 3.5 × 2.8 m. ([OLC](https://github.com/google/open-location-code))
- **what3words — exclude.** It is proprietary and the algorithm is patented. The API terms forbid reverse engineering and any use outside their API, so an offline, keyless implementation is not possible legally. ([OSM wiki](https://wiki.openstreetmap.org/wiki/What3words), [API licence](https://what3words.com/api-licence-agreement))

## 3. Wasm toolchain
- **Rust with wasm-bindgen is the best fit.** wasm-bindgen is at 0.2.128 (2026-09-04) and moved to its own GitHub org with new maintainers in July 2025. The old `rustwasm` org was archived and wasm-pack was sunset, so use `cargo build` plus `wasm-bindgen-cli` plus `wasm-opt` directly. ([Rust blog](https://blog.rust-lang.org/inside-rust/2025/07/21/sunsetting-the-rustwasm-github-org), [life after wasm-pack](https://nickb.dev/blog/life-after-wasm-pack-an-opinionated-deconstruction/))
- **Alternatives:**
  - Emscripten C++ only makes sense for reusing C/C++ code (GeographicLib, PROJ). It produces larger glue code.
  - AssemblyScript has a thin ecosystem for geodesy.
  - Pure TypeScript is simplest, but trig results differ between JavaScript engines (see determinism below).
- **Rust crates:**

| Crate | Version | What it gives you |
|---|---|---|
| `geo` | 0.33.1, MIT/Apache-2.0 | Buffer, BooleanOps (through i_overlay), Haversine, Rhumb, Karney geodesic, Densify, Simplify, ConvexHull, Validation |
| `geographiclib-rs` | 0.2.7, MIT | Subset of GeographicLib: geodesic direct/inverse and polygon area, pure Rust on `libm` |
| `i_overlay` | 8.1.2 | Polygon boolean ops and offsetting. Note: geo pins `i_overlay` ^4.5, not the 8.x line |
| `clipper2` | 0.6.0 | Alternative for boolean ops and offsetting |
| `rstar` | 0.13.0 | R-tree spatial index |
| `robust` | 1.2.0 | Shewchuk's exact geometric predicates |
| `proj` | 0.31.0 | Bindings to C PROJ. Heavy and hard to build for Wasm; prefer `proj4rs` [mem] or hand-written projections |

- **WASI:** WASI 0.3.0 was ratified 2026-06-11 and adds native async. Wasmtime 46 is the first release that implements the final spec; jco supports it. WASI 1.0 is targeted for late 2026 or early 2027. ([Bytecode Alliance](https://bytecodealliance.org/articles/WASI-0.3), [wasi.dev](https://wasi.dev/releases/wasi-p3))
  - For the MCP server, don't depend on the component model. The simplest route is to load the same wasm-bindgen `.wasm` from Node, Deno or Bun, all of which run standard Wasm plus JS glue. The `@modelcontextprotocol/sdk` is at 1.30.0.
- **Threads:** SharedArrayBuffer requires the page to be cross-origin isolated (COOP `same-origin` plus COEP `require-corp` or `credentialless`). Safari does not support `credentialless`, so every cross-origin tile or DEM fetch would need CORP or CORS headers. ([caniuse](https://caniuse.com/mdn-http_headers_cross-origin-embedder-policy_credentialless)) Use `wasm-bindgen-rayon` only as an optional second build chosen by feature detection. Fixed-width 128-bit SIMD is safe to use everywhere.
- **Determinism:**
  - Core Wasm floating point is IEEE-754 with no implicit FMA. The only nondeterminism is NaN sign and payload bits. The "deterministic profile" defines canonical NaNs. ([Nondeterminism.md](https://github.com/WebAssembly/design/blob/main/Nondeterminism.md))
  - **Avoid relaxed-SIMD.** Its FMA, reciprocal and min/max results can differ by CPU architecture. ([relaxed-simd](https://github.com/WebAssembly/relaxed-simd/blob/main/proposals/relaxed-simd/Overview.md))
  - JavaScript `Math.sin` and similar are not guaranteed to match across engines. V8 and SpiderMonkey use fdlibm ports; JavaScriptCore uses the system libm. ([macwright](https://macwright.com/2020/02/14/math-keeps-changing), [scrapfly](https://scrapfly.dev/posts/browser-math-os-fingerprint/))
  - So: do all trig inside Wasm with Rust's `libm` crate, never import `Math.*`, and pin the Rust toolchain. That gives bit-identical results in the browser and the MCP server. Add golden-vector tests that compare the bits.

## 4. Rendering
- **WebGPU support** ([gpuweb status](https://github.com/gpuweb/gpuweb/wiki/Implementation-Status)):

| Browser | Status |
|---|---|
| Chrome | Mac, Windows, ChromeOS since 113. Android since 121. Linux: Intel Gen12+ from 144, NVIDIA on Wayland from 147, other GPUs behind a flag |
| Safari 26 | On by default on macOS, iOS, iPadOS and visionOS |
| Firefox | Windows since 141. Apple Silicon Macs since 145/147. Intel Mac, Linux and Android still Nightly or behind a flag, with 2026 targets |

  Conclusion: WebGL2 fallback is mandatory.
- **deck.gl 9.4.0** (2026-09-05): every official layer is ported to WebGPU with render parity. The docs still call WebGPU experimental. luma.gl is at 9.4.1. ([deck.gl WebGPU](https://deck.gl/docs/developer-guide/webgpu))
- **MapLibre GL JS 6.10.0** (2026-09-15): v6 is ESM-only and requires WebGL2. Globe view arrived in 5.0. It is WebGL only; there is no WebGPU renderer. ([v6 migration](https://maplibre.org/maplibre-gl-js/docs/guides/v5-to-v6-migration-guide/))
- **Others:** three 0.186 (has WebGPURenderer, general purpose), CesiumJS 1.145 (a heavy full 3D globe), regl (WebGL1-era, lightly maintained [mem]).
- **For a lightweight retro-HUD canvas, use luma.gl directly.** It abstracts WebGPU and WebGL2 with one device API, has no map chrome and needs no API key. Draw vector geometry as line primitives. Add a post-process pass for the CRT phosphor look: bloom from a blurred bright pass, persistence by blending in the previous frame, scanlines, barrel distortion and a slight RGB offset. The basemap stays optional. Add MapLibre only if slippy tiles are wanted.
- **Basemaps:**
  - Natural Earth is public domain. 1:110m and 1:50m coastlines and borders are small enough to bundle.
  - Protomaps PMTiles: one file read with HTTP range requests (`pmtiles` 4.5.0). The planet is about 120 GB at zoom 0–15; `pmtiles extract --maxzoom` makes small regional cuts. ODbL requires OpenStreetMap attribution. ([Protomaps](https://docs.protomaps.com/basemaps/downloads))

## 5. Static hosting and PWA
- **Cloudflare Pages** supports a `_headers` file (max 100 rules, 2,000 characters per line), so COOP/COEP headers are possible. Cloudflare now steers new projects to Workers static assets; Pages is still supported with no forced migration. ([headers](https://developers.cloudflare.com/pages/configuration/headers/), [migration](https://developers.cloudflare.com/workers/static-assets/migration-guides/migrate-from-pages/))
- **GitHub Pages** has no custom headers. The only route to isolation is `coi-serviceworker`, which forces a reload on the first visit and conflicts with a Workbox service worker. ([coi-serviceworker](https://github.com/gzuidhof/coi-serviceworker))
- **Workbox** 7.4.1 is maintained by Chrome's Aurora team. Precache the app shell, the `.wasm` and the Natural Earth data; use cache-first for data the user opts in to.
- **Storage:**
  - In a Safari tab, all script-writable storage (IndexedDB, Cache API, service worker, OPFS) is wiped after 7 days of Safari use without visiting the site. Home-screen web apps are exempt.
  - The origin quota is up to about 60% of disk. Call `navigator.storage.persist()`.
  - Use OPFS for large DEM or PMTiles extracts.
  - ([WebKit](https://webkit.org/blog/14403/updates-to-storage-policy/), [MDN](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria))

## 6. Command palette search
uFuzzy's own benchmark, run on 162K entries:

| Library | Version | Size (minified) | Init | 86 searches |
|---|---|---|---|---|
| uFuzzy | 1.0.19 | 7.6 KB | 0.5 ms | 434 ms |
| fuzzysort | 4.0.2 | 6.2 KB | 50 ms | 1,321 ms |
| Fuse.js | 7.5.0 | 24.2 KB | 31 ms | 33,875 ms |

([uFuzzy](https://github.com/leeoniya/uFuzzy))

At about 800 entries every option finishes well under 1 ms per keystroke, so choose on ranking quality. uFuzzy or fuzzysort are both fine. Put aliases and units in the search text (for example "TAS", "density altitude", "geohash"), and add a recency boost.

## 7. Terrain / DEM
- **Copernicus GLO-30:** free, worldwide, with no time limit. Reproduction, distribution and modification are allowed, commercial use included, since nothing restricts it. Obligations:
  - Show this notice: "© DLR e.V. 2010-2014 and © Airbus Defence and Space GmbH 2014-2018 provided under COPERNICUS by the European Union and ESA; all rights reserved". Modified data uses the "produced using Copernicus WorldDEM-30 …" form.
  - Include the disclaimer that the organizations in charge of Copernicus accept no liability for any use of the data.
  - Don't imply endorsement.

  (Copernicus DEM licence, Articles 4 and 6, [licence PDF](https://docs.sentinel-hub.com/api/latest/static/files/data/dem/resources/license/License-COPDEM-30.pdf))
  - Source: `s3://copernicus-dem-30m` (eu-central-1). Cloud-optimized GeoTIFFs (COGs) with DEFLATE compression and 1024-pixel internal tiles. There are no ocean tiles, and some tiles are held back from the public release (reported exclusions include Armenia and Azerbaijan). ([readme](https://copernicus-dem-30m.s3.amazonaws.com/readme.html))
  - **The bucket sends no CORS header** (reported by [geodocs, Sept 2026](https://geodocs.io/en/blog/free-dem-elevation-data-sources); I did not test this myself). Browsers can't read it directly, so mirror or convert the tiles to your own CORS-enabled R2 or S3 bucket.
- **Mapzen/Tilezen Terrain Tiles** (`elevation-tiles-prod`, Terrarium PNG, normal, GeoTIFF and skadi formats): multi-source, with a long attribution list (USGS, ETOPO1, EU-DEM and others). ETOPO1 is marked "not for navigation." ([registry](https://registry.opendata.aws/terrain-tiles/), [attribution](https://github.com/tilezen/joerd/blob/master/docs/attribution.md)) Browser CORS on this bucket is widely used, but I did not re-verify it.
- **SRTM** is public domain (USGS) [mem]. Its gaps are worse than Copernicus.
- **Reading COGs:** `geotiff` 3.0.5 (geotiff.js) reads COGs with range requests. The server must allow CORS for `Range` and expose `Content-Range`.
- **Viewshed plan:** fetch a small DEM window, compute in Wasm, and cache it in OPFS for offline use.

## 8. Web Audio for UI sounds
[mem] Synthesize sounds with OscillatorNode, GainNode envelopes and noise buffers, so there are no sound files to ship. An AudioContext starts suspended until a user gesture, so create or `resume()` it on the first click or key press. Default sound to off or quiet, with a mute toggle saved in `localStorage`. `prefers-reduced-motion` has nothing to do with audio, but it is a reasonable proxy for turning off "juice": use it to disable flicker, persistence and shake, and treat it as a reason to keep sound off by default.

## Recommended stack
- **Core:** Rust workspace → a single `.wasm` built with wasm-bindgen, no threads, no relaxed-SIMD, trig from `libm`. Crates: h3o, geo, geographiclib-rs, rstar, robust, and a5-rs or OLC in Rust. It stays deterministic and gives one binary for both the browser and the MCP server.
- **MCP server:** Node (also runs on Deno and Bun) loading the same `.wasm`, with the official TypeScript SDK and stdio transport.
- **UI:** a TypeScript shell, uFuzzy for the palette, and luma.gl 9.4 on WebGPU with WebGL2 fallback for the HUD. Natural Earth is bundled; PMTiles is an optional layer.
- **Hosting:** Cloudflare (Pages, or Workers static assets for new projects) for header control, plus a Workbox precache, `storage.persist()`, and OPFS for downloaded data.
- **Terrain:** Copernicus GLO-30 converted to COGs or Terrarium tiles and hosted by you with CORS, read by geotiff.js.

## Risks
- Firefox on Linux, Intel Mac and Android, and many Linux Chrome GPUs, will hit the WebGL2 path, so the effects must degrade gracefully.
- deck.gl and luma.gl still call WebGPU experimental, so API churn is likely.
- Cross-origin isolation clashes with third-party tiles and Safari. That is why threads should stay optional.
- Safari tabs lose offline data after 7 days of Safari use without a visit. Prompt users to install the app.
- The Copernicus CORS gap means you pay for hosting and egress. The attribution and liability text is mandatory.
- h3o vs H3 4.5 parity (especially containment modes) is unconfirmed, and A5 is pre-1.0. Both need conformance tests against h3-js and a5-js golden outputs.
- wasm-pack is sunset; the wasm-bindgen maintainer handover adds some bus-factor risk.
- Aviation outputs need a clear "not for navigation" disclaimer. That is consistent with the ETOPO1 and Copernicus liability terms.
- Registry facts were checked live on 2026-09-18. Some web-fetch summaries returned 2024 dates that the GitHub API contradicted, so I used the API dates. Treat any [mem] item as needing a check.
