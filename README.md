# geoprims

**Geospatial and aerospace primitives.** Geodesy, navigation, aviation, drone, surveying, spatial-indexing, and terrain math, exact and cited, running entirely on your device.

- **For humans:** a fast static website at geoprims.com with a tactical "retro-HUD" canvas that draws every result.
- **For agents:** a local MCP server that runs the exact same calculators. Clone a release tag and run `node mcp/server.mjs`, with zero dependencies and no network; `npx -y @geoprims/mcp` also works.

No ads, no accounts, no tracking, no server-side compute. Inputs never leave the device.

> **Status:** specification only. No code has been written yet. This repository holds the product specs in [OpenSpec](https://github.com/Fission-AI/OpenSpec) format and the research behind them.

## What's specified

Planned inventory (targets from the specs; the live site will count only tools that have passed verification):

| Domain | Operations | Tool ids | Examples |
|---|---|---|---|
| Geodesy | 89 | 173 | DMS/MGRS/UTM/State Plane conversion, datums with epochs, geoid heights, WMM2025 declination |
| Navigation | 46 | 58 | Karney geodesics, rhumb lines, cross-track, fly-by turns, horizon and line of sight, 3D slant range |
| Geometry | 38 | 44 | Geodesic area, buffers, hulls, simplification, point-in-polygon, boolean operations |
| Aviation | 97 | 117 | ISA, CAS/TAS/Mach, density altitude, cold-temperature correction, E6B wind, W&B, METAR/TAF and winds-aloft decoding, holding entries, VDP |
| Drone | 49 | 49 | GSD, overlap and trigger timing, survey grids, endurance, VLOS guidance, lidar planning, link budget, dated Part 107/EASA references |
| Survey | 72 | 72 | Traverse closure, reductions, earthwork, curves, deed plotter, legacy land units, PLSS, GNSS field planning, ALTA precision |
| Indexing | 52 | 92 | H3 (full v4 API), S2, geohash, XYZ/TMS/quadkey tiles, Plus Codes, A5 |
| Raster | 30 | 40 | NDVI and other indices on local GeoTIFFs, elevation profiles, slope, terrain line of sight, viewshed |
| Time | 19 | 19 | Sun position, sunrise/sunset/twilight, the four legal "nights", Zulu, GPS week, Julian date |
| Units | 22 | 170 | Exact unit conversions (knots, nautical miles, inHg, US survey foot as legacy) |
| **Total** | **514** | **834** | |

An *operation* is a distinct calculation with its own test vectors. A *tool id* is anything you can open or call, including allow-listed conversion pairs built from operations (for example `dms-to-mgrs`). Search-friendly slugs such as `crosswind-calculator` are aliases and are not counted. About 300 tool pages are indexable. The rest open as presets of their parent tool, so search engines never see near-duplicates. Public counts include only tools that have passed the stable verification bar.

**Launch first, breadth second.** About 30 hero tools for pilots, drone operators, surveyors, and developers ship first, each reviewed, glance-tested, and mobile-gated ([launch plan](openspec/changes/plan-launch-and-value-proof/design.md)).

## Principles the specs enforce

- **One calculator, two surfaces.** A Rust → WebAssembly core. The website and the MCP server load byte-identical modules and return byte-identical results.
- **Reference accuracy.** Karney geodesics (not haversine by default), GeographicLib/PROJ/H3/S2/NOAA differential tests, and golden vectors from authoritative sources. A verification report is published with every release.
- **Explicit assumptions.** Datum realizations, epochs, height references, true vs magnetic, and rules of thumb vs exact values all surface as named warnings.
- **Honest time-sensitivity.** Magnetic models enforce their validity windows. NGS 2022 datums are labeled beta until adopted. FAA Part 108 is labeled "Proposed".
- **Private by construction.** Permalinks keep inputs in the URL fragment. The only network requests are same-origin static assets and coarse (≥ 1°) data tiles, plus a problem report if you choose to send one.
- **Glanceable.** Every tool opens with a real worked example and puts the answer first in a plain sentence ("Density altitude is 7,932 ft, about 2,900 ft higher than the field"), next to the rule of thumb and its error.
- **Show your work.** Every result has a "How we got this" panel with the formula using your numbers, cited sources with edition and section, assumptions, limitations, and a last-verified date. CI fails if a tool cites a superseded edition or an expired model.
- **Report a problem.** One button on every tool sends the exact inputs, outputs, and build to a small Cloudflare D1 inbox, after you preview the payload. There are no IPs, no accounts, and a public known-issues page.
- **Field-ready.** 48 px targets, numeric keypads that work, and sunlight and cockpit-safe night modes. It works offline and on a 320 px screen.
- **Accessible.** WCAG 2.2 AA in every theme. Audio is off by default. Reduced motion is respected.

## Build order

Each change in [`openspec/changes/`](openspec/changes/) has a proposal, a design, specs (except the planning change), and a task list.

| # | Change | Delivers |
|---|---|---|
| 1 | [`establish-platform-foundation`](openspec/changes/establish-platform-foundation/) | Tool contract, Wasm core, determinism, units, data assets, verification, privacy, catalog |
| 2 | [`define-build-contracts`](openspec/changes/define-build-contracts/) | Route map, page anatomy and notices, manifest fields, codes registry, reference profiles, report API, Cloudflare topology |
| 3 | [`build-web-experience`](openspec/changes/build-web-experience/) | Static site, command palette, HUD canvas, themes, import/export, offline PWA, docs |
| 4 | [`add-trust-and-proof`](openspec/changes/add-trust-and-proof/) | Citations, standards freshness ledger, correctness program, proof panel |
| 5 | [`add-glanceable-and-field-ux`](openspec/changes/add-glanceable-and-field-ux/) | Answer-first anatomy, plain-language sentences, mobile input contract, field and night modes |
| 6 | [`add-problem-reporting`](openspec/changes/add-problem-reporting/) | Report button, Cloudflare Worker + D1, triage, known issues |
| 7 | [`add-seo-and-discoverability`](openspec/changes/add-seo-and-discoverability/) | Indexable page rules, structured data, OG images, sitemaps, llms.txt, natural-language prefill |
| 8 | [`add-local-mcp-server`](openspec/changes/add-local-mcp-server/) | Zero-dependency local MCP server, clone-and-run, six meta-tools |
| 9 | [`add-geodesy-suite`](openspec/changes/add-geodesy-suite/) | Parsing, frames, datums, projections, grid references, heights, geomagnetism |
| 10 | [`add-navigation-and-geometry`](openspec/changes/add-navigation-and-geometry/) | Geodesics, routes, line of sight, 3D vectors, computational geometry |
| 11 | [`add-aviation-suite`](openspec/changes/add-aviation-suite/) | Atmosphere, airspeed, altimetry, wind/E6B, performance, fuel and loading |
| 12 | [`add-drone-suite`](openspec/changes/add-drone-suite/) | Photogrammetry, mission patterns, endurance, operations references |
| 13 | [`add-survey-suite`](openspec/changes/add-survey-suite/) | COGO and traverse, reductions, earthwork, alignment curves |
| 14 | [`add-spatial-indexing-and-raster`](openspec/changes/add-spatial-indexing-and-raster/) | H3/S2/geohash/tiles/Plus Codes, spectral indices, terrain analysis |
| 15 | [`add-practitioner-essentials`](openspec/changes/add-practitioner-essentials/) | Sun and time, weather decoding, IFR geometry, deeds and PLSS, GNSS field tools, drone sensors and links |
| 16 | [`plan-launch-and-value-proof`](openspec/changes/plan-launch-and-value-proof/) | Phases, hero tools, launch bar, tracking-free value metrics, cut list |

Changes 1–8 are the platform (Phase 0), with `define-build-contracts` authoritative wherever two specs meet. Domain changes 9–14 can proceed in parallel once geodesy (9) lands, because the others reuse its parsing, frames, and heights. Practitioner essentials (15) comes last because it builds on every suite. Change 16 sets the order of work: hero tools first.

```bash
openspec list
```

```bash
openspec validate --all --strict
```

## Building it

[AGENTS.md](AGENTS.md) holds the working rules for anyone, human or AI, implementing the specs: the three doors (web, MCP, report), how to add a tool, and the non-negotiables.

## Research

The specs cite seven research briefs gathered on September 18, 2026. Items marked unverified still need a primary-source check during implementation.

- [Geodesy libraries, datums, and reference data](docs/research/01-geodesy-and-reference-data.md)
- [Spatial indexing, Wasm toolchain, rendering, and hosting](docs/research/02-indexing-wasm-rendering-hosting.md)
- [Aviation, drone, surveying, and remote-sensing formulas](docs/research/03-aviation-drone-survey-formulas.md)
- [MCP, competitors, legal, and accessibility](docs/research/04-mcp-competitors-legal-a11y.md)
- [Feedback backend, SEO, mobile and field UX, trust patterns, MCP distribution](docs/research/05-feedback-seo-ux-trust-mcp-distribution.md)
- [Patterns reused from roughlogic.com](docs/research/06-roughlogic-patterns.md)
- [Practitioner gap analysis, hero tools, and journeys](docs/research/07-practitioner-gap-analysis.md)

## Disclaimer

geoprims tools are planning, engineering, and education aids. They are not certified for navigation and are not legal survey determinations. Verify operational values against official sources, your aircraft's POH/AFM, and current regulations.

## License

[MIT](LICENSE)
