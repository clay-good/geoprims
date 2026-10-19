## Why

"Put points where I should take measurements" is a job every domain here has: soil and crop sampling under a drone survey, ground control and checkpoints spread evenly, a fishnet for a lidar bid, transects for a wildlife count, a sampling grid for an environmental site. Today geoprims generates one kind of grid (the H3 cover) and one layout (GCP suggestions). Everything else means QGIS or ArcGIS, and "random" there is not reproducible, so a plan cannot be re-run or audited.

This change adds a small sampling toolbox: fishnets and hexagonal grids in a projected CRS, random, systematic, and stratified points inside a polygon from a seed, transect lines, and points along a line. Every design cites its statistical source, every random layout is reproducible from its seed on both surfaces, and every point is a lat/lon row ready to export.

Depends on: `add-navigation-and-geometry` (polygons, triangulation, geodesic area), `add-geodesy-suite` (projections).

## What Changes

Adds the `geometry.sampling` group, seven operations:

| Tool | Question it answers | Source |
|---|---|---|
| `geometry.sampling.fishnet` | Cut this area into cells of a given size, aligned to a grid | QGIS "Create grid" conventions; ArcGIS "Create Fishnet" terminology |
| `geometry.sampling.hex-grid` | Cover this area with hexagons of a given width | QGIS "Create grid" (hexagon); USFS FIA hexagonal design as the field example |
| `geometry.sampling.random-points` | Give me N random points inside this polygon, reproducibly | Cochran 1977 ch. 2; EPA QA/G-5S ch. 3; Osada et al. 2002 for the triangle method |
| `geometry.sampling.systematic-points` | Give me a regular grid of points with a random start | EPA QA/G-5S ch. 5 (square and triangular grids) |
| `geometry.sampling.stratified-points` | Give me k points in each cell of a grid over this polygon | Cochran 1977 ch. 5; EPA QA/G-5S ch. 4 |
| `geometry.sampling.transects` | Lay parallel (or zigzag) transects across this area at a spacing and bearing | Buckland et al. 2001 and 2015 (line-transect design) |
| `geometry.sampling.points-along-line` | Put points every d meters along this line | QGIS "Points along geometry"; GRASS `v.to.points` conventions |

Randomness comes from xoshiro256** seeded by splitmix64 from a user-visible integer seed, so the same seed gives the same points on the website and the MCP server, byte for byte.

## Capabilities

### New Capabilities

- `geometry/sampling`: Grid generation, seeded random and systematic sampling, transects, and points along lines.

### Modified Capabilities

None.

## Non-goals

- Sample-size calculations or statistical analysis of results.
- Adaptive, ranked-set, or cluster designs.
- Routing between points (a later change could order points for a field visit).

## Impact

- `core/gp-geometry` grows by the sampling operations and a seeded PRNG (the first and only random source in the core, used by no other tool).
- `data/taxonomy.json` adds the `sampling` group under `geometry`.
- New sources-ledger rows: Cochran 1977, EPA QA/G-5S (2002), Osada et al. 2002, Blackman & Vigna 2021 (xoshiro), Buckland et al. 2001 and 2015, USFS GTR SRS-80 (2005), the QGIS and ArcGIS grid documentation (vendor).
