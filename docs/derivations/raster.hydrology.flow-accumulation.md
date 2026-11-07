<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Flow accumulation over a DEM (`raster.hydrology.flow-accumulation`)

## Method

Once every cell knows where its water goes, accumulation says how much ground drains through each one. A cell counts itself plus every cell upstream of it; multiplied by the area of a cell, that is the contributing area, and it is what picks channels out of a hillside: a stream is where the accumulation passes a threshold.

**D8** sends a cell's whole count to the neighbour of steepest descent. The sum is taken in topological order (Kahn's method): every cell with nothing draining into it is ready, and passing its count downstream makes its receiver one step closer to ready. Each cell is therefore added exactly once, in a single sweep, and no cell is counted before everything above it is in.

**D-infinity** (Tarboton 1997) splits a cell's flow between the two neighbours its angle lies between, in proportion to how close the angle is to each. Since the split is by angle rather than by cell, counts are no longer whole numbers. Cells are summed from the highest down, which puts every contribution in before a cell passes its own on.

A no-data cell neither receives nor passes water. A pit, having no lower neighbour, keeps everything that reaches it, which is why conditioning comes first; the result counts the pits it found and warns.

## Equations

- D8: `a(i) = 1 + Σ a(j)` over every cell j whose steepest descent points at i.
- D-infinity: `a(i) = 1 + Σ p(j→i)·a(j)`, where `p(j→i)` is the share of j's angle that falls on i: with the angle between the directions k and k+1 at 45° apart, `p = 1 − frac` to k and `frac` to k+1, `frac` being the angle's position between them.
- Area through a cell = `a(i) × s²`, with s the cell size.

## Symbols and units

Elevations and cell size in meters (any length unit is accepted); accumulation is a count of cells, whole under D8 and fractional under D-infinity; area in square meters.

## Domain

- 2 to 300 rows and columns and at most 25,000 cells, else `LIMIT_EXCEEDED`; cell size more than zero.
- Interior cells with no lower neighbour trap what reaches them, counted and warned with `PITS_PRESENT`.
- The window is the whole world to this tool: a cell counts only what drains to it from inside the grid. Near the edge that is a lower bound, which the result says in words.

## Approximations

D8's count is exact for its own rule but the rule itself is a staircase: a hillslope that faces between two of the eight directions sends all its water one way, which is why accumulation maps from D8 show parallel stripes on smooth ground. D-infinity spreads the flow and looks smoother, at the price of counts that are no longer cells you can point at. Neither resolves flats. The grid is plane: cell area is the cell size squared, so a geographic grid needs its cell size in meters at the latitude.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: A 10 by 10 ramp falling to the east
- sourceEdition: add-raster-hydrology, raster/hydrology, the East ramp accumulation scenario
- sourceLocator: The spec's own scenario: each row reads 1, 2, 3 … 10 from west to east, counting the cell itself, which is the reference behaviour the change names
- independent: no for this example, which is the specification's scenario rather than a published one. What stands in for it is the differential test below: a second implementation, which walks each cell's water downhill one step at a time instead of sweeping in topological order, agrees with the core on every one of the 19 vectors, including the whole accumulation grid on eight of them.
- inputs: the ramp, cell size 30 m, method d8
- outputs: at most 10 cells drain through one cell, which is 9,000 m², at row 1 column 10
- tolerance: exact on the counts, 1e-9 on the area
- verifiedBy: golden vectors v001 and v002 and `core/crates/gp-raster/tests/hydrology.rs` `flow_accumulation_scenarios`
- verifiedOn: 2026-11-06

Parity with WhiteboxTools `D8FlowAccumulation` and richdem, which the change's task 2 asks for, is still pending: both live in the differential container, which this machine cannot build.

## Differential tests

- `tools/vectors/gen_flowaccum.py` builds the 19 vectors in `core/vectors/raster.hydrology.flow-accumulation.jsonl`. Its D8 oracle uses a different method from the core's: it walks every cell's water downhill one step at a time, needing no ordering at all, so the two agree only if both are right. Its D-infinity oracle sums from the highest cell down with Tarboton's proportional split. The cases are the spec's east ramp by both methods, ramps south and south-east, a valley that gathers along its floor, an outward-draining cone, a pit, a no-data hole, six seeded random terrains, and two refusals; eight of them pin the whole accumulation grid row by row.
- Pending: WhiteboxTools and richdem, which the change's reference container pins (task 2). Neither could be installed on this machine, so no parity figure is claimed.

## Invariants

- `core/crates/gp-raster/tests/hydrology.rs` `flow_accumulation_scenarios`: the spec's east ramp reads 1 to 10 across every row, the largest area is 9,000 m² at the east column, and D8 and D-infinity agree cell for cell on that planar ramp, where there is nothing to split.
- `core/crates/gp-raster/tests/hydrology.rs` `flow_accumulation_invariants`: the conservation identity at every cell of a cone, taken against the pointer grid the flow-direction tool reports, so the two tools are checked against each other; every count is between 1 and the number of cells; and a pit collects all nine cells of a 3 by 3 and carries `PITS_PRESENT`.
