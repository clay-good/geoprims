# geoprims

**Geospatial and aerospace primitives.** Geodesy, navigation, aviation, drone, surveying, spatial-indexing, and terrain math, exact and cited, running entirely on your device.

- **For humans:** a fast static website at geoprims.com with a tactical "retro-HUD" canvas that draws every result.
- **For agents:** a local MCP server (`npx -y @geoprims/mcp`) that runs the exact same calculators.

No ads, no accounts, no tracking, no server-side compute. Inputs never leave the device.

> **Status:** specification only. No code has been written yet. This repository holds the product specs in [OpenSpec](https://github.com/Fission-AI/OpenSpec) format and the research behind them.

## What's specified

| Domain | Operations | Endpoints | Examples |
|---|---|---|---|
| Geodesy | 89 | 173 | DMS/MGRS/UTM/State Plane conversion, datums with epochs, geoid heights, WMM2025 declination |
| Navigation | 46 | 58 | Karney geodesics, rhumb lines, cross-track, fly-by turns, horizon and line of sight, 3D slant range |
| Geometry | 38 | 44 | Geodesic area, buffers, hulls, simplification, point-in-polygon, boolean operations |
| Aviation | 84 | 112 | ISA, CAS/TAS/Mach, pressure and density altitude, cold-temperature correction, E6B wind, W&B |
| Drone | 42 | 52 | GSD, overlap and trigger timing, survey grids, hover power and endurance, dated Part 107/EASA references |
| Survey | 58 | 68 | Traverse closure and adjustment, instrument reductions, earthwork volumes, horizontal and vertical curves |
| Indexing | 52 | 92 | H3 (full v4 API), S2, geohash, XYZ/TMS/quadkey tiles, Plus Codes, A5 |
| Raster | 30 | 40 | NDVI and other indices on local GeoTIFFs, elevation profiles, slope, terrain line of sight, viewshed |
| Units | 22 | 170 | Exact unit conversions (knots, nautical miles, inHg, US survey foot as legacy) |
| **Total** | **461** | **809** | |

An *operation* is a distinct calculation with its own test vectors. An *endpoint* is an addressable tool, including allow-listed conversion pairs built from operations (for example `dms-to-mgrs`). Both numbers are reported, and public counts include only tools that have passed the stable verification bar, so the count is not padded.

## Principles the specs enforce

- **One calculator, two surfaces.** A Rust → WebAssembly core. The website and the MCP server load byte-identical modules and return byte-identical results.
- **Reference accuracy.** Karney geodesics (not haversine by default), GeographicLib/PROJ/H3/S2/NOAA differential tests, and golden vectors from authoritative sources. A verification report is published with every release.
- **Explicit assumptions.** Datum realizations, epochs, height references, true vs magnetic, and rules of thumb vs exact values all surface as named warnings.
- **Honest time-sensitivity.** Magnetic models enforce their validity windows. NGS 2022 datums are labeled beta until adopted. FAA Part 108 is labeled PROPOSED.
- **Private by construction.** Permalinks keep inputs in the URL fragment. The only network requests are same-origin static assets and coarse (≥ 1°) data tiles.
- **Accessible HUD.** WCAG 2.2 AA in every theme. Audio is off by default. Reduced motion is respected.

## Build order

Each change in [`openspec/changes/`](openspec/changes/) has a proposal, a design, specs, and a task list.

| # | Change | Delivers |
|---|---|---|
| 1 | [`establish-platform-foundation`](openspec/changes/establish-platform-foundation/) | Tool contract, Wasm core, determinism, units, data assets, verification, privacy, catalog |
| 2 | [`build-web-experience`](openspec/changes/build-web-experience/) | Static site, command palette, HUD canvas, theme, audio, import/export, offline PWA, docs |
| 3 | [`add-local-mcp-server`](openspec/changes/add-local-mcp-server/) | Local MCP server with discovery-first meta-tools, toolsets, and release gating by agent evals |
| 4 | [`add-geodesy-suite`](openspec/changes/add-geodesy-suite/) | Parsing, frames, datums, projections, grid references, heights, geomagnetism |
| 5 | [`add-navigation-and-geometry`](openspec/changes/add-navigation-and-geometry/) | Geodesics, routes, line of sight, 3D vectors, computational geometry |
| 6 | [`add-aviation-suite`](openspec/changes/add-aviation-suite/) | Atmosphere, airspeed, altimetry, wind/E6B, performance, fuel and loading |
| 7 | [`add-drone-suite`](openspec/changes/add-drone-suite/) | Photogrammetry, mission patterns, endurance, operations references |
| 8 | [`add-survey-suite`](openspec/changes/add-survey-suite/) | COGO and traverse, reductions, earthwork, alignment curves |
| 9 | [`add-spatial-indexing-and-raster`](openspec/changes/add-spatial-indexing-and-raster/) | H3/S2/geohash/tiles/Plus Codes, spectral indices, terrain analysis |

Changes 1–3 are the platform. Domain changes 4–9 can proceed in parallel once geodesy (4) lands, because the others reuse its parsing, frames, and heights.

```bash
openspec list
```

```bash
openspec validate --all --strict
```

## Research

The specs cite four research briefs gathered on September 18, 2026. Items marked unverified still need a primary-source check during implementation.

- [Geodesy libraries, datums, and reference data](docs/research/01-geodesy-and-reference-data.md)
- [Spatial indexing, Wasm toolchain, rendering, and hosting](docs/research/02-indexing-wasm-rendering-hosting.md)
- [Aviation, drone, surveying, and remote-sensing formulas](docs/research/03-aviation-drone-survey-formulas.md)
- [MCP, competitors, legal, and accessibility](docs/research/04-mcp-competitors-legal-a11y.md)

## Disclaimer

geoprims tools are planning, engineering, and education aids. They are not certified for navigation and are not legal survey determinations. Verify operational values against official sources, your aircraft's POH/AFM, and current regulations.

## License

[MIT](LICENSE)
