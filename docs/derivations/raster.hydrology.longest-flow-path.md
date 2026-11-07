<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Longest flow path to a point (`raster.hydrology.longest-flow-path`)

## Method

The hydraulic length a time-of-concentration formula asks for is the longest way water travels to the outlet. On a D8 grid that is the longest chain of pointers ending at the chosen cell. The tool finds it by walking the pointers upstream from the outlet and keeping, at each cell, the longest branch above it; the answer is read back down from the outlet, so the path comes out from its top end to the outlet.

Its length adds the cell size for a step to a side neighbour and √2 times the cell size for a step to a corner, measuring between cell centres. The drop is the path's first cell above the outlet, and the average slope is that drop over that length. The path comes back twice: as its cells, with the elevation at each, and as vertices measured east and south of the grid's north-west cell, which carry latitudes and longitudes too when that cell's place is given.

## Equations

- Longest length at a cell i: `L(i) = max over tributaries u of (L(u) + d(u→i))`, with `d` the cell size or `√2 ×` it.
- Drop = `z(first cell of the path) − z(outlet)`.
- Average slope = drop / length.
- A vertex of a cell: `east = (column + ½)·s`, `south = (row + ½)·s`.

## Symbols and units

Elevations, cell size, length and drop in meters (any length unit is accepted); average slope as rise over run, dimensionless; rows and columns from 1 at the north-west; latitudes and longitudes in degrees.

## Domain

- 2 to 300 rows and columns and at most 25,000 cells, else `LIMIT_EXCEEDED`; cell size more than zero. A full window measured through `dist/wasm` on this machine takes 4.7 ms at the 95th percentile.
- The outlet's row and column are required, are at least 1 by the fields' own limits, and must lie inside the window on a cell with an elevation; otherwise `OUT_OF_DOMAIN` naming the window's size.
- The north-west corner comes as both latitude and longitude, or neither.
- An interior pit cuts a path short, counted and warned with `PITS_PRESENT`.

## Approximations

The path follows the D8 staircase, so across a slope facing between two of the eight directions it reads longer than the water's real line: the classic case is a diagonal valley, where every step is a corner step and the path is √2 times the straight distance. The window bounds the answer: a path reaching the edge is as much as the data holds. Lengths are plane, from the cell size, so a geographic grid needs its cell size in meters at the latitude. The placement in latitude and longitude uses fixed degree lengths (111,132 m of latitude; 111,320 m times the cosine of the grid's latitude for longitude), the contour tool's convention.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: The path to the east edge of a ramp of 30 m cells
- sourceEdition: add-raster-hydrology, raster/hydrology, the Ramp path scenario
- sourceLocator: The spec's own scenario: the longest path to the east edge of a ten-cell east ramp with 30 m cells is 270 m, and the path is the row
- independent: no for this example, which is the specification's scenario rather than a published one. What stands in for it is the differential test below: a second implementation, which walks every cell downhill and keeps the longest path that passes the outlet, agrees with the core's upstream walk on every one of the 15 vectors.
- inputs: the 10 by 10 east ramp, cell size 30 m, outlet row 1 column 10
- outputs: 270 m, a 9 m drop, 10 cells, starting at row 1 column 1
- tolerance: 0.001 m on the length, 1e-9 m on the drop, exact on the cells
- verifiedBy: golden vector v001 and `core/crates/gp-raster/tests/hydrology.rs` `longest_flow_path_scenarios`
- verifiedOn: 2026-11-07

Parity with WhiteboxTools `LongestFlowpath`, which the change's task 2 asks for, is still pending: it lives in the differential container, which this machine cannot build.

## Differential tests

- `tools/vectors/gen_longestpath.py` builds the 15 vectors in `core/vectors/raster.hydrology.longest-flow-path.jsonl` from a second implementation that works the other way round: it walks every cell's own path downhill and keeps the longest one that passes the outlet. The cases are the spec's ramp at two outlets, a plane falling south-east where every step is diagonal, a valley at its foot and at its junction, a valley draining south at two outlets, an outward-draining cone, a grid with a no-data hole, three seeded random terrains, and two refusals; three of them pin the path cell by cell.
- Pending: WhiteboxTools, which the change's reference container pins (task 2); it could not be installed on this machine, so no parity figure is claimed.

## Invariants

- `core/crates/gp-raster/tests/hydrology.rs` `longest_flow_path_scenarios`: the spec's ramp gives 270 m, a 9 m drop, ten cells, a path that is the row itself, and an average slope equal to the drop over the length.
- `core/crates/gp-raster/tests/hydrology.rs` `longest_flow_path_invariants`: this tool and the watershed tool report the same path, cell for cell, for the same outlet; on a plane falling south-east the length is the step count times √2 cells; the path ends at the outlet and never climbs; and with the grid's corner given every vertex carries a place.
