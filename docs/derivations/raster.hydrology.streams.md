<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Stream network and Strahler order (`raster.hydrology.streams`)

## Method

A channel, on a DEM, is a cell with enough ground draining through it. That is the whole model: pick a threshold in cells or in area, and the cells whose D8 accumulation reaches it are the network. Everything else here follows from that choice, which is why the threshold is an input and not a default.

The channel cells form a graph: each one sends its water to the neighbour of steepest descent, and the pointer counts where that neighbour is also a channel cell. **Strahler order** (1957) is then read off the graph from the top down. A channel cell with no channel cell draining into it is order 1, a headwater. Where channels meet, two of the same order make the next order up, and a tributary of lower order leaves the order it joins alone. The cells are ordered in topological sweep, so every tributary is ordered before the cell it joins.

A **link** is what a reader means by "a stream": the stretch between one junction and the next. It begins at a source or a junction, follows the pointers down, and ends at the next junction or where the network leaves the window. Its length adds the cell size along a side step and √2 times it along a diagonal. Its outlet area is the accumulation at its last cell times the area of a cell: the ground draining through the bottom of that stretch.

The links come back with their cell centres as vertices, measured east and south of the grid's north-west cell. Give the north-west cell's latitude and longitude and every vertex carries a place as well, which is what makes the network drawable on a map.

## Equations

- Channel: `acc(i) ≥ threshold`, with the threshold in cells, or an area divided by the cell area.
- Strahler: `order(i) = 1` with no channel tributary; `max` of the tributaries' orders, plus 1 when two or more share that maximum.
- Link length = `Σ s` over side steps and `Σ √2·s` over diagonal steps, with s the cell size.
- Outlet area of a link = `acc(last cell) × s²`.
- A vertex of a cell: `east = (column + ½)·s`, `south = (row + ½)·s` from the north-west cell's corner.

## Symbols and units

Elevations and cell size in meters (any length unit is accepted); the threshold in cells or in square meters; lengths in meters; areas in square meters; latitudes and longitudes in degrees.

## Domain

- 2 to 300 rows and columns and at most 25,000 cells, else `LIMIT_EXCEEDED`; cell size more than zero.
- The threshold is required, as cells or as an area, and never both; it is at least one cell.
- The north-west corner comes as both latitude and longitude, or neither.
- When no cell reaches the threshold the network is empty, which is an answer rather than an error: `EMPTY_NETWORK` says so and the result gives the most that drains through any cell, so the reader can pick a threshold that finds something.
- An interior pit ends a channel in the middle of the grid; `PITS_PRESENT` says how many, and the fill tool comes first.

## Approximations

The threshold is the model of what a channel is, and nothing here picks it: a different threshold is a different network, as the vectors at four thresholds on the same grid show. Links are the cells themselves, so a length is a walked path of cell centres and reads longer than a smooth channel, most of all where the D8 path staircases across a slope. The window is all the tool knows, so a stream that enters from outside begins at the edge with whatever accumulation the window gives it, which is a lower bound. The placement of vertices in latitude and longitude uses fixed degree lengths (111,132 m of latitude; 111,320 m times the cosine of the grid's latitude for longitude), which is the contour tool's convention and good to a part in a thousand over a window this size.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: Two headwaters joining into one stream
- sourceEdition: add-raster-hydrology, raster/hydrology, the Order rule scenario
- sourceLocator: The spec's own scenario: two first-order links join, and what leaves the junction is second order
- independent: no for this example, which is the specification's scenario rather than a published one. What stands in for it is the differential test below: a second implementation, which orders the network by repeated passes rather than in one topological sweep, agrees with the core on every one of the 20 vectors.
- inputs: the 7 by 7 Y-shaped grid, cell size 30 m, threshold 3 cells
- outputs: 9 channel cells, 3 links, highest order 2, two first-order links and one second-order link
- tolerance: exact on orders, counts and links; 1e-9 on lengths and areas
- verifiedBy: golden vectors v001 to v004 and `core/crates/gp-raster/tests/hydrology.rs` `stream_order_rule`
- verifiedOn: 2026-11-06

Parity with WhiteboxTools `StrahlerStreamOrder`, which the change's task 2 asks for, is still pending: it lives in the differential container, which this machine cannot build.

## Differential tests

- `tools/vectors/gen_streams.py` builds the 20 vectors in `core/vectors/raster.hydrology.streams.jsonl` from a second implementation written in Python. Its ordering works differently from the core's: it passes over the network repeatedly, ordering whatever has all its tributaries already ordered, rather than sweeping once in topological order, so the two agree only if both are right. The cases are the spec's order rule at four thresholds, a threshold no cell reaches, a valley at three thresholds, an outward-draining cone at two, a valley with a no-data hole, six seeded random terrains, and three refusals. Six of them pin every link's order, cell count and outlet area.
- Pending: WhiteboxTools, which the change's reference container pins (task 2); it could not be installed on this machine, so no parity figure is claimed.

## Invariants

- `core/crates/gp-raster/tests/hydrology.rs` `stream_order_rule`: the spec's two headwaters join into a second-order link; the links' lengths add up to the network length; every link carries a positive outlet area and at least one cell; and the vertices are exactly the cells the links run through.
- `core/crates/gp-raster/tests/hydrology.rs` `stream_threshold_and_placement`: a threshold past the window's own maximum gives an empty network with `EMPTY_NETWORK` and the maximum stated; raising the threshold never adds channel cells; the threshold as an area gives the same network as the same threshold in cells; both thresholds at once, neither, and half a corner are refused; and with the north-west corner given, a vertex sits south and east of it.
