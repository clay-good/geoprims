<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Fill or breach the pits in a DEM (`raster.hydrology.fill`)

## Method

Flow work on a DEM assumes water can leave every cell. A measured DEM does not oblige: noise, bridges, culverts, and road embankments leave depressions that trap water, and a flow-direction routine run over them produces basins that end nowhere. Conditioning removes those depressions before any of the flow tools see the grid.

**Filling** uses Priority-Flood with an ε slope (Barnes, Lehman and Mulla 2014, Algorithm 3). Every cell on the window's edge, and every cell that touches a no-data cell, is an outlet: water leaves the data there. Those cells seed a priority queue ordered by elevation, lowest first. The lowest cell is popped and each of its unvisited neighbours is examined: a neighbour at or below that cell plus ε is raised to exactly that height, so the path back out descends by ε a step, and then the neighbour is pushed with its new elevation. Because the queue is always drained from the lowest cell outward, a depression is filled from its spill point inward, and the water surface it ends with is the one the terrain would hold.

**Breaching** (Lindsay 2016's hybrid) tries to cut a channel out of each pit rather than drown it. From each pit, a Dijkstra search spreads outward over a cost surface: the cost of entering a cell is the earth that must be dug from it to carry water at the pit's level, less ε a step down the channel, and a cell that needs no digging costs nothing. The search stops at the first cell lower than the pit, and the cells along that path are cut to descend by ε. A pit with no outlet within the depth and length allowed is left, and Priority-Flood then fills it, with `BREACH_INCOMPLETE` saying how many. That final filling pass is what makes it a hybrid rather than a breaching method.

No-data cells are never raised, never cut, and never crossed: they are boundaries, and the conditioned grid has exactly the no-data cells the one entered had.

## Equations

- Filling, for a cell n reached from cell c: `z(n) ← max(z(n), z(c) + ε)`.
- Breaching, for the k-th cell of a channel out of a pit at `z_p`: `z(k) ← min(z(k), z_p − k·ε)`, and the cost of reaching it is `Σ max(0, z(k) − (z_p − k·ε))`.
- Fill volume = `Σ (z_after − z_before) · s²` over raised cells, with s the cell size.
- Cut volume = `Σ (z_before − z_after) · s²` over lowered cells.
- A pit is an interior cell with `z(i) < z(n)` for all eight neighbours n. A cell on the window's edge or beside no data is never a pit, because water leaves the data there.

## Symbols and units

Elevations and the cell size in meters (any length unit is accepted); ε in meters, 1e-5 by default; volumes in cubic meters; the deepest cut and longest channel in meters and cells. The grid is plane: cell area is the cell size squared, so a geographic grid in degrees needs its cell size given in meters at its latitude.

## Domain

- 2 to 300 rows and columns, and at most 25,000 cells in one window, else `LIMIT_EXCEEDED`. A full window measured through `dist/wasm` on this machine takes 24.7 ms at the 95th percentile to fill and 57.1 ms to breach, against the 10 ms budget the benchmark holds the primary example to.
- Cell size more than zero; ε zero or more; the deepest cut zero or more.
- ε of zero is allowed and leaves flats: every filled cell sits at its spill elevation. That is what some published filling routines do, and the result says which ε was used.
- The window's edge is an outlet. A depression that continues past the edge is treated as draining there, so clip wider than the basin of interest.

## Approximations

The ε slope is an alteration of the terrain, not a measurement: it is small enough not to disturb a slope map and large enough to carry a flow routine out of a filled basin. Breaching cuts a channel one cell wide and does not widen it, so the earth it reports is a lower bound on what a real excavation would move. With ε of zero the filled surface is flat, and flow routines need their own flat-resolution step.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: A 7 by 7 grid with one pit 3 m deep
- sourceEdition: add-raster-hydrology, raster/hydrology, the Single bowl scenario
- sourceLocator: The spec's own scenario: a bowl whose floor is 95 m inside rims of 98, 99, and 100 m, filled to the 100 m rim with an ε step inward
- independent: no for this example, which is the specification's scenario rather than a published one. What stands in for it is the differential test below: a second implementation of both published algorithms, written in Python from the papers, agrees with the core on every one of the 22 vectors to 1e-6.
- inputs: the 7 by 7 bowl, cell size 10 m, method fill
- outputs: 25 cells changed, 3,700.035 m³ filled, the deepest raise 5.00003 m, 1 pit before and 0 after
- tolerance: 1e-6 m³ and 1e-6 m
- verifiedBy: golden vector v001 and `core/crates/gp-raster/tests/hydrology.rs` `fill_scenarios`
- verifiedOn: 2026-11-06

Parity with WhiteboxTools and richdem, which the change's task 2 asks for, is still pending: both oracles live in the differential container, which this machine cannot build.

## Differential tests

- `tools/vectors/gen_hydrology.py` builds the 22 vectors in `core/vectors/raster.hydrology.fill.jsonl` from a second implementation of Priority-Flood + ε and of the hybrid breach, written in Python from the papers with `heapq`, sharing no code with the core. The two agree to 1e-6 m³ and 1e-6 m on every case: the spec's two scenarios, a no-data hole, a plane that already drains, nested depressions, a ridge holding back a basin at three settings, eight seeded random terrains at four sizes and both methods, two values of ε, and two refusals.
- Pending: WhiteboxTools `FillDepressions` and `BreachDepressionsLeastCost`, and richdem, which the change's reference container pins (task 2). Neither could be installed on this machine, so no parity figure is claimed.

## Invariants

- `core/crates/gp-raster/tests/hydrology.rs` `fill_scenarios`: the spec's Single bowl (25 cells raised to the spill, no edge cell moved, every interior cell left with a descending path to the edge) and Pit at the edge stays (nothing moved, because the edge is an outlet).
- `core/crates/gp-raster/tests/hydrology.rs` `fill_invariants`: a no-data cell is never changed; the fill volume equals the sum of the rises in the changes list times the cell area; conditioning is idempotent, so the conditioned grid conditions to itself with nothing changed; and a surface that already drains is left alone by both methods.
- `core/crates/gp-raster/tests/hydrology.rs` `breaching_cuts_instead_of_filling`: on a basin held back by a ridge, breaching moves less earth than filling and still leaves no pit; with a cut too shallow to reach through the ridge, the pit is filled instead and `BREACH_INCOMPLETE` says so.
- `core/crates/gp-raster/tests/hydrology.rs` `windows_past_the_cap_are_refused`: 40,000 cells are refused with `LIMIT_EXCEEDED`.
