<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Flow direction over a DEM (`raster.hydrology.flow-direction`)

## Method

Where does the water on this cell go? **D8** (O'Callaghan and Mark 1984) answers with one of the eight neighbours: the one the steepest descent leads to, slope being the drop divided by the distance, which is the cell size to a side and √2 times it to a corner. One pointer per cell is what every downstream routine needs, because it makes the grid a graph that can be walked.

**D-infinity** (Tarboton 1997) answers with an angle instead. The 3 by 3 window around a cell holds eight triangular facets, each spanned by the centre cell, one cardinal neighbour, and one diagonal neighbour. On each facet the downslope vector is (s₁, s₂) from the two edges; its direction is atan2(s₂, s₁), clamped into the facet when it points outside, and its magnitude is the slope. The steepest facet wins, and its local angle becomes a global one by Tarboton's equation (6), rg = a_f·r + a_c·π/2, with the a_c and a_f of his Table 1. The result is an angle measured counterclockwise from east, which splits flow between two neighbours instead of committing to one. The tool still reports the D8 pointer beside it, as published implementations do, so the grid can be walked.

The codes are the three tables practitioners actually read: ESRI's powers of two (E 1, SE 2, S 4, SW 8, W 16, NW 32, N 64, NE 128), WhiteboxTools's clockwise base-two table starting at NE 1 (so E 2 and NW 64, as its help page gives), and TauDEM's 1 through 8 counterclockwise from east. A cell with no lower neighbour gets 0 in all three.

## Equations

- D8 slope to a neighbour: `s = (z₀ − zₙ) / d`, with `d` the cell size for a side neighbour and `√2 ×` it for a corner. The direction is the neighbour of greatest positive `s`; a tie goes to the first in ESRI's order, east and clockwise.
- D-infinity, per facet (Tarboton equations 1 to 6): `s₁ = (e₀ − e₁)/d₁`, `s₂ = (e₁ − e₂)/d₂`, `r = atan2(s₂, s₁)`, `s = √(s₁² + s₂²)`; if `r < 0` then `r = 0, s = s₁`; if `r > atan2(d₂, d₁)` then `r = atan2(d₂, d₁)`, `s = (e₀ − e₂)/√(d₁² + d₂²)`. The global angle is `rg = a_f·r + a_c·π/2`.
- On this grid the cells are square, so `d₁ = d₂` and a facet's far edge is at π/4.

## Symbols and units

Elevations and cell size in meters (any length unit is accepted); slopes are rise over run, dimensionless; D-infinity angles in radians counterclockwise from east, with −1 where no facet descends. Direction codes are integers from the chosen table, 0 for no outflow.

## Domain

- 2 to 300 rows and columns and at most 25,000 cells, else `LIMIT_EXCEEDED`; cell size more than zero.
- A cell with no lower neighbour has no direction. On the window's edge, or beside no data, that means the water leaves the data, and the result counts it under cells draining out of the window. Inside, it means a pit, counted separately and warned with `PITS_PRESENT`: condition the grid with the fill tool first.
- No-data cells carry the no-data value in the direction and angle grids, never a direction.

## Approximations

D8 commits a whole cell's water to one of eight directions, so on a hillslope that faces, say, 20° east of south, the flow follows a staircase rather than the slope: that is the dispersion problem Tarboton's paper opens with, and the reason D-infinity exists. D-infinity's angle is faithful but cannot be walked cell to cell, which is why watershed and longest-path work stay on D8. Neither method resolves flats: with no descent there is no steepest descent, and the published flat-routing methods (Garbrecht and Martz 1997) are a separate step this tool does not do.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: A 10 by 10 ramp falling to the east
- sourceEdition: add-raster-hydrology, raster/hydrology, the Ramp and Encoding-option scenarios
- sourceLocator: The spec's own scenarios: every cell of an east-falling ramp points east, which is 1 in the ESRI table and 2 in the WhiteboxTools table, and the D-infinity angle is 0 radians
- independent: no for this example, which is the specification's scenario rather than a published one. What stands in for it is the differential test below: a second implementation of both published rules, written in Python from the papers, agrees with the core on every one of the 22 vectors, including the whole pointer grid on nine of them.
- inputs: the ramp, cell size 30 m, method d8, encoding esri
- outputs: 90 cells with a direction, every one 1 (east); the 10 cells of the east column drain out of the window; steepest descent 0.0333…
- tolerance: exact on the codes and counts, 1e-9 on the slope
- verifiedBy: golden vectors v001 to v004 and `core/crates/gp-raster/tests/hydrology.rs` `flow_direction_scenarios`
- verifiedOn: 2026-11-06

Parity with WhiteboxTools `D8Pointer` and `DInfPointer`, which the change's task 2 asks for, is still pending: both live in the differential container, which this machine cannot build.

## Differential tests

- `tools/vectors/gen_flowdir.py` builds the 22 vectors in `core/vectors/raster.hydrology.flow-direction.jsonl` from a second implementation written in Python from the papers, sharing no code with the core: the spec's ramp in all three encodings and by D-infinity, ramps the other three ways, a plane falling south-east, Tarboton's outward-draining cone by both methods and in TauDEM codes, an unconditioned pit, a no-data hole, six seeded random terrains, and two refusals. Nine of them pin the whole pointer grid row by row; the rest pin the counts and the steepest slope.
- Pending: WhiteboxTools and richdem, which the change's reference container pins (task 2). Neither could be installed on this machine, so no parity figure is claimed.

## Invariants

- `core/crates/gp-raster/tests/hydrology.rs` `flow_direction_scenarios`: the spec's Ramp in all three encodings, with the east column draining off the grid, and the D-infinity angle of 0 radians on the same ramp.
- `core/crates/gp-raster/tests/hydrology.rs` `flow_direction_invariants`: every code produced is one of its table's eight or zero; a ramp turned around turns the direction, east and west being opposite codes; D8 and D-infinity agree on a plane falling south-east, which is code 2 and 7π/4; and an unconditioned pit reports a cell with no outflow and carries `PITS_PRESENT`.
