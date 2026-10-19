## Context

Research gathered 2026-10-19: QGIS and ArcGIS grid conventions, EPA QA/G-5S design definitions, Cochran's designs, Osada's triangle sampling, and the xoshiro and PCG generator papers. The geometry module has Delaunay triangulation, geodesic area, and planar predicates on robust arithmetic, and the geodesy module has every projection a local grid needs.

## Goals / Non-Goals

**Goals:**
- Reproducible layouts: a seed and the inputs fully determine every point, on every host.
- Grids that match what QGIS and ArcGIS users expect, with the alignment rules stated.

**Non-Goals:**
- Statistics on the samples. Field routing.

## Decisions

### S1. One seeded generator, specified to the bit
Randomness uses xoshiro256** (Blackman & Vigna 2021) with state seeded by splitmix64 from the user's 64-bit integer seed, and doubles drawn as `(x >> 11) × 2⁻⁵³`. The generator lives in `gp-base` behind a feature only the sampling tools enable; the import lint still forbids any host random source. The first 10 outputs for seed 0 and seed 42 are pinned as vectors against the authors' CC0 reference C code.
- *Alternative: PCG32.* Equally good; it needs two draws per double. Either would do; one is pinned.
- *Alternative: no seed, host randomness.* Rejected: it breaks determinism and the both-surfaces gate.

### S2. Uniform points in a polygon by triangulation, not rejection
The polygon (with holes) is triangulated, a triangle is chosen with probability proportional to its area by binary search on cumulative areas, and a point is placed by Osada et al. 2002 §4.2: P = (1 − √r₁)A + √r₁(1 − r₂)B + √r₁r₂C. The draw count is exactly 2N, so seeds are portable and the MCP and web results are identical. Bounding-box rejection was rejected because its draw count depends on the polygon's shape.

### S3. Grids are built in a projected plane and returned geographic
Fishnets, hex grids, systematic points, and transects are generated in a user-chosen or automatically chosen projected CRS (the polygon's UTM zone by default, or a local azimuthal equidistant plane for areas crossing zones), then converted to lat/lon. The origin is the lower-left of the area's bounding box unless the reader gives one; an optional rotation turns the grid about the origin. The CRS, origin, and rotation are echoed in the result so the grid can be regenerated elsewhere. Cells are clipped to the polygon when "clip" is chosen, and partial cells are flagged with their inside fraction.

Hexagon geometry follows QGIS: flat-top hexagons of width w (horizontal spacing w, vertical spacing 0.866 w), rows offset by w/2.

### S4. Designs as EPA QA/G-5S defines them
- **Simple random:** N points, S2.
- **Systematic:** a square or triangular grid of spacing d with a random start offset drawn from the seed (EPA QA/G-5S ch. 5); "no random start" places the first point at the origin.
- **Stratified:** strata are fishnet cells (or hex cells) clipped to the polygon; k points per stratum by S2 within each stratum, allocating by stratum order so the seed stream is reproducible.
Every result names the design, the source, N, the seed, and the area each point represents.

### S5. Transects and points along lines
Transects are parallel lines at a spacing and bearing across the polygon, clipped to it, with a random start offset from the seed (Buckland et al. 2015 design conventions); "zigzag" joins consecutive transect ends. Points along a line are placed every d meters along the planar length in the chosen CRS, starting at a start offset, and the last point is kept only if it lies within the line (QGIS convention); the geodesic length is reported beside the planar one so the difference is visible.

### S6. Limits
At most 100,000 points, 50,000 cells, or 10,000 transects per call; `LIMIT_EXCEEDED` names the limit and the count the inputs would produce, computed before allocation. Polygons follow the geometry vertex limits.

## Tool inventory

| Tool | Endpoints | Reference | Independent vector |
|---|---|---|---|
| fishnet | 1 | QGIS Create grid; ArcGIS Create Fishnet | QGIS `qgis_process` output for a 100 m grid over a test polygon |
| hex-grid | 1 | QGIS Create grid; USFS GTR SRS-80 | QGIS output for a 50 m hex grid |
| random-points | 1 | Cochran ch. 2; EPA QA/G-5S ch. 3; Osada 2002 | xoshiro reference outputs; a NumPy reimplementation of S2 with the same stream |
| systematic-points | 1 | EPA QA/G-5S ch. 5 | QGIS regular points with a fixed start |
| stratified-points | 1 | Cochran ch. 5; EPA QA/G-5S ch. 4 | the NumPy reimplementation |
| transects | 1 | Buckland 2001, 2015 | R `dssd` systematic parallel design on a test region |
| points-along-line | 1 | QGIS Points along geometry | QGIS output on a test line |

Total: 7 operations, 7 endpoints. The geometry rollup moves from 38 to 45 operations.

## Risks / Trade-offs

- **[Oracles are GPL (QGIS, R dssd)]** → Run only in `verify/` as external processes; nothing is copied.
- **[A planar grid on a large area misleads]** → Above 500 km extent the tool returns `LIMIT_EXCEEDED` with the reason; below it, the CRS is named and the maximum scale distortion is reported.
- **[Seed portability across implementations]** → The derivation note publishes the exact draw order; the NumPy oracle reimplements it.

## Migration Plan

Not applicable. New tools start experimental.

## Open Questions

None.
