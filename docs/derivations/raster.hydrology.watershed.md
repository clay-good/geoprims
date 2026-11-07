<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Watershed above a pour point (`raster.hydrology.watershed`)

## Method

A watershed is the answer to "what drains to here". Given a conditioned grid and a point, the tool finds every cell whose D8 path reaches that point (Jenson and Domingue 1988). It does it from the outlet outward: build the list of cells draining into each cell, then walk that list upstream from the outlet until nothing new is reached. Every cell visited is in the basin.

Before delineating, the point is **snapped**, because a point one cell off the channel delineates a hillside rather than a basin. Two rules, both named in the result: the nearest channel cell at a stream threshold (the default, Jenson and Domingue's rule), or the cell with the most water within the search distance. `none` leaves the point where it is, which is what a reader wants when the point is already a cell of the channel.

From the basin come its measures: the area is the cell count times the cell area, the relief is the basin's highest cell above the outlet, and the mean slope averages each basin cell's steepest descent. The **longest flow path** is the longest chain of pointers ending at the outlet, found by the same upstream walk, keeping at each cell the longest branch above it; it comes back as its cells from the top down, with its length and the drop along it, which is what a time-of-concentration estimate needs.

## Equations

- Basin = `{ i : the D8 path from i reaches the outlet }`.
- Area = `cells × s²`, with s the cell size.
- Relief = `max z over the basin − z(outlet)`.
- Mean slope = the mean of `(z(i) − z(next))/d` over the basin's cells, d being s or `√2·s`.
- Longest path length at a cell = `max over tributaries u of (length(u) + step(u→i))`, with `step` s or `√2·s`.

## Symbols and units

Elevations, cell size, relief, path length and drop in meters (any length unit is accepted); area in square meters; mean slope as rise over run, dimensionless; rows and columns counted from 1 at the north-west.

## Domain

- 2 to 300 rows and columns and at most 25,000 cells, else `LIMIT_EXCEEDED`; cell size more than zero. A full window measured through `dist/wasm` on this machine takes 7.9 ms at the 95th percentile, the snap and the longest path included.
- The pour point needs both its row and its column, and must lie inside the window and on a cell with an elevation; otherwise `OUT_OF_DOMAIN` naming the window's size.
- When the basin includes a cell on the window's edge, the result says so and raises `WATERSHED_TOUCHES_EDGE`: the real basin may continue beyond the data.
- An interior pit cuts a basin short, counted and warned with `PITS_PRESENT`.

## Approximations

The window is the world: a basin that reaches the edge is as much of the basin as the data holds, and the area is a lower bound. The area is plane, the cell count times the cell size squared, not a geodesic area, so a geographic grid needs its cell size in meters at the latitude. Mean slope is the average of the cells' own steepest descents rather than a fitted surface, which reads a little steeper than a plane fit on rough ground. The longest flow path follows the D8 staircase, so it reads longer than a smooth channel. The default stream threshold for snapping, when none is given, is a thousandth of the window's cells and at least five; the result states where the outlet ended up either way.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: The basin above the bottom of a valley
- sourceEdition: add-raster-hydrology, raster/hydrology
- sourceLocator: The change's own grid: two headwaters feeding a stem, with the outlet at the stem's foot
- independent: no for this example, which is the change's own scenario rather than a published one. What stands in for it is the differential test below, where a second implementation walking every cell downhill agrees with the core's upstream walk on every one of the 18 vectors, and the spec's own Ramp-path case (270 m along a 10-cell ramp of 30 m cells) is one of them.
- inputs: the 7 by 7 Y-shaped grid, cell size 30 m, pour point row 7 column 4, unsnapped
- outputs: the basin's cells and area, its relief above the outlet, its mean slope, and a longest flow path ending at the outlet
- tolerance: exact on cells, rows and columns; 1e-9 on lengths and areas; 1e-4 on the mean slope
- verifiedBy: golden vectors v001 and v002 and `core/crates/gp-raster/tests/hydrology.rs` `watershed_scenarios`
- verifiedOn: 2026-11-06

Parity with WhiteboxTools `Watershed`, `JensonSnapPourPoints` and `LongestFlowpath`, which the change's task 2 asks for, is still pending: they live in the differential container, which this machine cannot build.

## Differential tests

- `tools/vectors/gen_watershed.py` builds the 18 vectors in `core/vectors/raster.hydrology.watershed.jsonl` from a second implementation that works the opposite way round: it walks every cell's own path downhill and keeps the cells whose path passes the outlet, where the core walks the pointers upstream from the outlet. It also reimplements both snapping rules. The cases are the valley outlet and a point upstream of it (the nested-basin scenario), the same point off the channel under both snap rules, the spec's Ramp path at two rows, a valley at three outlets, an outward-draining cone, a valley with a no-data hole, four seeded random terrains, and three refusals; four of them pin the basin grid cell by cell.
- Pending: WhiteboxTools, which the change's reference container pins (task 2); it could not be installed on this machine, so no parity figure is claimed.

## Invariants

- `core/crates/gp-raster/tests/hydrology.rs` `watershed_scenarios`: the nested-basin scenario, where the basin of an upstream cell lies entirely inside the basin below it and is smaller; the area is the cell count times the cell area; the longest path ends at the outlet; and the spec's Ramp path is 270 m with a 9 m drop.
- `core/crates/gp-raster/tests/hydrology.rs` `watershed_edges_and_refusals`: a basin reaching the edge says so and raises `WATERSHED_TOUCHES_EDGE`; snapping moves a point off the channel onto it, says it moved, and finds a larger basin than the same point unsnapped; and a pour point off the window or on no data is refused with `OUT_OF_DOMAIN` naming the window's size.
