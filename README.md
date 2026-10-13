# geoprims

**Exact, cited math for maps, flight, drones, and surveying. It runs on your device.**

Density altitude before a flight. Ground sample distance for a drone survey. A traverse closure, a UTM conversion, an H3 cell, sunrise at a field site. geoprims answers questions like these in a plain sentence, then shows how it got there.

- **For people:** a fast, free website. The answer comes first, with a map or diagram and a "How we got this" panel showing the formula with your numbers and its sources.
- **For AI agents:** a local MCP server that runs the same calculators and returns the same results, byte for byte. It has no dependencies and stays offline unless optional verified asset downloads are explicitly enabled. See [mcp/README.md](mcp/README.md).

## Why trust it

- **Accurate.** Reference methods (Karney geodesics, not haversine), tested against GeographicLib, PROJ, H3, S2, and published worked examples, each held to a per-domain tolerance (1 mm in geodesy, for example) unless the source itself prints coarser.
- **Cited.** Every result names its source, edition, and section. The build fails if a source goes out of date. Every citation link is checked monthly, and a dead or moved one opens an issue.
- **Honest.** Assumptions like datum, true vs. magnetic north, and rules of thumb show up as warnings, not fine print.
- **Private.** No ads, accounts, tracking, or server-side math. Your inputs never leave your device.
- **Auditable.** The [CycloneDX software bill of materials](data/sbom.cdx.json) lists every locked Cargo and npm dependency and its license. The build fails when either lockfile and the inventory disagree.

## What it covers

Geodesy, navigation, geometry, aviation, drones, surveying, spatial indexing, raster and terrain, time, and units.

The 8 launch journeys are published as end-to-end workflows with prefilled, linked calculator steps, and the concept library exceeds its initial 25-explainer launch target. The build runs every workflow through the shared core and verifies every explainer's sources, live example, and reciprocal tool links.

Spatial indexing includes H3 polygon fills with all four H3 v4 containment modes, including holes, antimeridian crossings, and the library's polar edge cases. The map renders fills approaching 1,000,000 cells through view-selected H3 parent levels, then restores finer detail as you zoom. The experimental tools for A5, pinned to version 0.10.1, index points into equal-area pentagons and expose their boundaries and hierarchy.

Geometry includes planar and geodesic-edge polygon intersection, union, difference, and symmetric difference. The planar engine matches GEOS across 400 generated degenerate pairs and 48 polygon pairs from JTS 1.20.0's canonical area-overlay suites, through all four operations.

Terrain contours accept either a local meter grid or a north-up geographic grid. Geographic grids use latitude-aware WGS 84 cell dimensions, return map coordinates for every contour vertex, and draw each contour as a separate polyline while preserving the same contourpy-verified topology as local grids. Terrain curvature is stable for a 3 x 3 window, with its Zevenbergen-Thorne sign and scale checked against xDEM 0.2.3. Terrain line of sight is stable for user-supplied profiles, including Earth curvature, refraction, and Fresnel clearance; its hidden/visible boundary is checked against the published GRASS GIS 8.5.0 viewshed fixture.

The generated catalog currently exposes 31 navigation operations and 15 geometry operations. All 46 are stable, with a source-backed derivation, at least 20 golden vectors, and a generated tool page. The [haversine vs. geodesic explainer](https://geoprims.com/learn/geodesic-vs-haversine/) compares spherical, ellipsoidal, Vincenty, and rhumb methods with worked examples. The remaining catalog expansion is tracked in [`add-navigation-and-geometry`](openspec/changes/add-navigation-and-geometry/tasks.md); its practitioner terms include “as the crow flies,” “great circle,” “XTE,” and “loxodrome.”

The US 1976 atmosphere is verified through its 86 km ceiling: every kilometer through 81 km is cross-checked against an independent implementation, and every printed 0.5 km row from 82 to 86 km is checked against NOAA-S/T 76-1562 Table I.

GPS satellite visibility and DOP planning is stable for pasted YUMA or SEM almanacs. Its almanac propagation and geometry are covered by 27 golden vectors, including Orekit 13.1.9's published comparison with Trimble Planning.

GNSS site localization is stable for 4-parameter similarity and 6-parameter affine fits. It recovers IOGP Guidance Note 7-2's published method 9621 scale and rotation, and 24 live vectors plus coefficient and translation invariants cover residuals, mixed units, suspect scale, degenerate control, and compact networks at large coordinate origins.

Mission sortie planning is stable for ordered waypoint paths. It counts every trip from home and back before choosing battery-swap waypoints, and 22 vectors cover flight-time and battery-energy inputs, reserves, wind, distinct survey and transit speeds, and an unreachable-path refusal.

Multirotor hover power is stable for separate or coaxial rotors, explicit aerodynamic and electrical losses, and site air density. Its momentum-theory result is checked against the 2026 Embry-Riddle aerospace textbook's 45,000 lb tiltrotor example and 19 additional vectors.

Test-flight hover calibration is stable for measured energy and time, avionics draw, assumed electrical efficiency, and site air density. It back-solves the hover model's figure of merit, reverses Embry-Riddle's published tiltrotor example, and has 25 vectors plus four round-trip and unit invariants.

Payload impact is stable for payload mass, payload electrical draw, usable energy, aircraft losses, and site air density. Its mass-to-power relationship reproduces a published fit to measured payload-hover tests, and 25 vectors plus four invariants cover power, endurance, units, boundaries, and invalid loads.

Maximum payload is stable for a target hover time, usable energy, avionics draw, aircraft losses, and site air density. It inverts the same measured payload-hover fit to recover a 50 g load, and 25 vectors plus five invariants cover inverse power, units, scaling, atmosphere, boundaries, and infeasible targets.

Return-to-home energy is stable for an along-track wind, distance home, cruise power, remaining energy, and a landing reserve. It reproduces the FAA's published 134.4 mph minus 30 mph headwind example, and 25 vectors plus five invariants cover return time, energy, margin, round-trip reach, units, wind boundaries, and invalid inputs.

Mission VLOS checking is stable for a reader-entered range or the shared EASA size-and-visibility guidance. It measures every waypoint with WGS84 geodesics, builds the map ring, and has 25 GeographicLib-backed vectors plus two invariants covering list consistency, boundaries, units, polar and antimeridian routes, and invalid inputs.

Thermal pixel footprint is stable for direct IFOV or pixel-pitch and focal-length inputs, with an inverse range for a target size. It reproduces FLIR's published E8 example at 10 ft, and 26 vectors plus four invariants cover units, pixel rules, inverse geometry, domain boundaries, and invalid camera inputs.

Side slope and grade conversion is stable across percent, per mille, degrees, decimal grades, and explicitly labeled H:V or V:H ratios. It reconstructs rise, run, or slope length, refuses ambiguous bare ratios, and reproduces FHWA's 20 m over 400 m roadway example across 23 vectors.

Vertical-curve sight distance is stable for crest and headlight-sag geometry, including both the sight-line-within-curve and sight-line-beyond-curve cases. It reproduces FHWA's published 49.8 m crest-curve result within the source's rounded-input tolerance, and 28 vectors cover metric and US units, cited design K comparisons, physical bounds, and required inputs.

Unequal-tangent vertical curves are stable for two parabolas that meet smoothly under the PVI, including crest, sag, monotone, straight, and tangent-extension cases. The tool reproduces Pearson's published problem 25.15 elevations, and 20 vectors cover metric and US stationing, endpoint turning points, unit guards, and invalid lengths.

The aviation catalog covers all 83 planned operations through 90 tool ids: 83 operation tools and 7 narrower airspeed endpoints. Altimeter inputs accept inHg, hPa, `A` groups, and `Q` groups directly rather than multiplying identical pages by unit.

## Status

Live at [geoprims.com](https://geoprims.com) since September 23, 2026, and still growing. Tools that pass full verification are marked stable; the rest are labeled experimental. Aggregate, tracking-free adoption and trust signals are recorded in the [value log](docs/value-log.md). The plan and specs live in [`openspec/changes/`](openspec/changes/).

## Build and run

You need [rustup](https://rustup.rs), binaryen's `wasm-opt`, and Node 22 or newer.

```bash
npm run build
```

```bash
node mcp/server.mjs
```

To contribute, start with [AGENTS.md](AGENTS.md).

Run `npm test` after building to check Rust and JavaScript behavior. The website has two more suites, both run in CI: `npm test` and `npm run test:browser` in `apps/web`, after `npm run build` there. The browser suite checks what unit tests cannot, such as diagram labels inside their frames and each page's JavaScript budget. The 50-route cold-load sweep currently tops out at 87.9 KB of compressed shell JavaScript against the 90 KB limit; export code loads on demand, and maps paint with Canvas2D before the optional GPU compositor loads. Runtime cancellation and shutdown behavior is documented in [packages/runtime/README.md](packages/runtime/README.md).

After changing a dependency lockfile, run `npm run build:sbom` to regenerate the dependency inventory used by the site’s licenses page.

Maintainers prepare and verify tagged deployments with the [release runbook](docs/runbooks/releases.md). The release workflow deploys first, then publishes attested GitHub assets; the MCP server is never sent to a package registry.

## Disclaimer

geoprims is a planning and education aid. It is not certified for navigation and is not a legal survey. Check operational values against official sources, your aircraft's POH/AFM, and current regulations.

## License

[MIT](LICENSE)
