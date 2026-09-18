## Context

Motivation is in `proposal.md`. Research: `docs/research/02-indexing-wasm-rendering-hosting.md`. Key facts (September 2026):

- **H3** core is v4.5.0 (Apache-2.0). `h3o` is a pure-Rust rewrite (BSD-3, v0.11), and its parity with 4.5 containment modes is unconfirmed.
- **S2.** The Rust `s2` crate is incomplete; `s2js` (TypeScript) and Go `golang/geo` are complete ports.
- **A5** (Apache-2.0) is pre-1.0 and has a Rust port.
- **Open Location Code** has official Rust and JS implementations plus shared test data.
- **Copernicus GLO-30:**
  - The license permits redistribution with mandatory attribution and a liability disclaimer.
  - The public S3 bucket sends no CORS headers, so browsers cannot read it directly. We re-host.
  - Some areas are excluded from the public release.
- **Local imagery.** `geotiff.js` reads COGs in browsers, but raster decoding is also possible in Rust (the `tiff` crate).

## Goals / Non-Goals

**Goals:**
- Bit-exact index parity with the reference implementations.
- Terrain and imagery processing entirely on the device, with honest data provenance.

**Non-Goals:**
- Imagery search or download. Hydrology. Global terrain analysis beyond the declared radius limits.

## Decisions

### IR1. H3 via h3o, with a C fallback per function
Use `h3o` for size and determinism. Run the H3 C differential suite over every function and resolution. Any function without parity (candidates: experimental containment modes) is served by H3 C compiled to Wasm for that function only, inside the same indexing module. It is replaced once h3o reaches parity.

Alternative: compile all of H3 C with Emscripten. Rejected for binary size and glue, though it stays the reference in CI.

### IR2. S2: port the needed subset from Go
Port cell ID math, cell geometry, neighbors, and RegionCoverer from `golang/geo` (Apache-2.0) into Rust, verified against S2 C++ outputs. The existing Rust crate is used where it already has parity.

### IR3. Plus Codes and geohash: small, native
Use the official OLC Rust implementation (Apache-2.0). Geohash is implemented directly (about 150 lines) with neighbor tables and tested against published vectors.

### IR4. Terrain hosting and tiling
Copernicus GLO-30 COGs are converted to 1° × 1° tiles as lossless float32 GeoTIFF (Deflate with floating-point predictor), preserving source values exactly, with per-tile digests. They are served from `assets.geoprims.com` with CORS and Range support. Terrain derivatives read tiles plus a 1-cell margin, so edge cells need no neighbor tile.

Alternative: Terrarium PNG tiles. Rejected: multi-source attribution complexity and a mixed "not for navigation" ETOPO lineage.

### IR5. Viewshed algorithm
R2-style radial sweep with a documented error bound, validated against brute-force line of sight on reference DEMs. It runs in a worker with progress and cancel. XDraw was considered: it is faster, with larger error near boundaries.

### IR6. Raster decoding in Rust
Decode GeoTIFF/COG in the raster module (`tiff` crate plus GeoKey parsing) to keep one deterministic path shared with the MCP server. Tiled reads keep memory under the declared caps.

## Tool inventory (targets)

| Domain | Group | Operations | Examples |
|---|---|---|---|
| indexing | `h3` | 26 | latlng-to-cell, cell-to-latlng, cell-boundary, resolution, base-cell, is-pentagon, is-class-iii, validate, grid-disk, grid-ring, grid-path, grid-distance, local-ij, parent, children, center-child, child-position, compact, uncompact, polygon-to-cells, cells-to-polygon, directed-edges, vertexes, cell-area, edge-length, resolution-table |
| indexing | `a5` | 3 | lonlat-to-cell, cell-boundary, hierarchy (experimental) |
| indexing | `s2` | 8 | point-to-cell, token, inspect, vertices, hierarchy, neighbors, area, cover |
| indexing | `geohash` | 4 | encode, decode, neighbors, cover |
| indexing | `tile` | 7 | latlon-to-tile, tile-bounds, xyz-tms, tile-quadkey, tile-hierarchy, bbox-tiles, ground-resolution |
| indexing | `pluscode` | 3 | encode, decode, shorten-recover |
| indexing | `cross` | 1 | cross-index (matched cell size) |
| **indexing** | | **52 ops / 92 endpoints** | +40 generated cross-index pairs (e.g. `h3-to-s2`, `geohash-to-h3`, `quadkey-to-tile`, `latlon-to-geohash`, `pluscode-to-latlon`) |
| raster | `index` | 12 | ndvi, ndwi-mcfeeters, ndwi-gao, mndwi, evi, evi2, savi, ndbi, nbr, dnbr, band-math, reflectance-scaling |
| raster | `terrain` | 18 | elevation, elevation-hae, profile, slope, aspect, hillshade, contours, line-of-sight, fresnel-terrain, viewshed, dem-info, dem-stats, roughness-tri, tpi, curvature, dem-difference-volume, highest-point, sampling |
| **raster** | | **30 ops / 40 endpoints** | +10 sensor-specific forms (e.g. `ndvi-sentinel-2`, `ndvi-landsat-8`, `nbr-sentinel-2`) |

## Risks / Trade-offs

- **[h3o parity gaps]** → Per-function C fallback plus the differential suite.
- **[Terrain hosting costs and data exclusions]** → R2 (no egress fees), coarse tiles, offline packs. Coverage gaps are reported explicitly.
- **[GLO-30 is a DSM; users expect bare earth]** → Stated in every result. User DEMs (e.g. USGS 3DEP bare-earth) are supported.
- **[Large local rasters exhaust memory]** → Tiled processing and declared megapixel caps.

## Migration Plan

Not applicable. A5 tools graduate from experimental when A5 reaches 1.0 and conformance passes.

## Open Questions

None that affect the specs.
