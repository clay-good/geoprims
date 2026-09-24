<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# S2 cells covering a region (`indexing.s2.covering`)

## Method

S2 divides the sphere by projecting it onto the six faces of a cube and subdividing each face as a quadtree, thirty levels deep. Covering a region means choosing a set of those cells whose union contains it.

The choice is a heuristic, and this is the first thing to say plainly: **a covering is not unique**. Starting from the six faces, the coarsest candidate that meets the region is split into its four children; a cell the region contains whole is kept as it is; a cell that only partly overlaps is kept rather than dropped, which is what makes the union contain the region. Refinement continues until the budget is reached. Google's own `RegionCoverer` uses a priority order instead, splitting the cell that will reduce the covered area most, and for the same region and budget it returns a different — often tighter — set. Both are correct coverings.

Two constraints bound the search. `min_level` sets the coarsest cell allowed and `max_level` the finest. `max_cells` is the budget, and the lowest level wins over it: if the region needs more cells than the budget just to reach `min_level`, the extra cells are returned and a `COVERING_OVER_BUDGET` warning says so, because a smaller set would leave part of the region out. Past four times the budget the request is refused instead, since returning tens of thousands of cells to a caller who asked for a few is not a better answer than saying so.

## Equations

There is no closed form. The procedure is:

- Start with the six face cells that meet the region.
- While the count is under the budget and the finest cell is coarser than `max_level`: take a cell the region does not contain whole, replace it with its four children, and drop children that miss the region.
- Stop at `max_level`, or when the budget is reached; never return a cell coarser than `min_level`.
- Covered area = the sum of the cells' areas, which is at least the region's.

## Symbols and units

Give a rectangle (`south`, `north`, `west`, `east`) or a circle (`lat`, `lon`, `radius`). `min_level` and `max_level` bound the cell size, `max_cells` the count. Out come `count`, the `cells` as tokens, `covered_area`, and the `finest_level` and `coarsest_level` actually used.

## Domain

Any rectangle or circle. Levels 0 to 30. West greater than east means the rectangle crosses the antimeridian.

## Approximations

The covering is a superset, never a tight fit, and the tool's accuracy line says so. How much larger depends on the shape and the budget: a region that lines up with the cell grid covers tightly, one that cuts across it does not.

## Worked example

- sourcePublisher: Google; s2sphere contributors
- sourceTitle: S2 Geometry Library; s2sphere, a Python implementation of the S2 algorithms
- sourceEdition: s2sphere 0.2.5
- sourceLocator: the S2 cell hierarchy and `RegionCoverer`'s contract — a covering contains its region, uses cells within the level range, and treats the cell count as a budget the lowest level can override
- independent: yes
- inputs: 22 regions, including ten rectangles and six circles over every face of the cube, both poles, the antimeridian, and budgets from 8 to 96
- outputs: the cell count, the levels used, and the first and last cell
- tolerance: exact
- verifiedBy: golden vectors v001 to v022, and 128 containment checks in `tests/s2_cover_parity.rs`, run by the core on every build
- verifiedOn: 2026-09-23

The independence here needs stating precisely, because the obvious comparison is the wrong one. Asking s2sphere's `RegionCoverer` for the same region and demanding the same cells would compare two heuristics and fail for a reason that is not a defect — they genuinely differ, and both are right. So the vectors pin this implementation's choice, as a regression guard, and the *verification* is a different question put to s2sphere: for 128 points it places inside these regions, it supplies the token of that point's own cell at every level the covering may use. A covering contains the point exactly when it holds one of those tokens, which is a set intersection — so the check never consults the core's own idea of where a cell is. All 128 pass.

Building that check found the budget behaviour worth documenting: the core returns 29 cells for a budget of 8 when `min_level` forces it, and says so in a warning. That is deliberate and correct — the alternative is a covering that does not cover — but "how many cells at most" is now qualified in the limitations rather than left to be discovered.

## Differential tests

- `core/crates/gp-indexing/tests/s2_cover_parity.rs`: 16 regions and 128 interior points from s2sphere, checking containment, the budget and the level range
- `core/crates/gp-indexing/tests/s2_parity.rs`: the cell geometry itself against s2sphere at 400 cases, levels 0 to 30
- `core/vectors/indexing.s2.covering.jsonl`: 22 vectors pinning this implementation's covering

- `tools/vectors/gen_s2_cover_polygon.py` and `core/crates/gp-indexing/tests/s2_cover_parity.rs` `every_polygon_covering_contains_its_polygon` (1.1.0): ten polygons with great-circle edges (across the antimeridian, on the equator, near a pole, from 200 m to 400 km, four with holes), each with 24 points inside it chosen by a great-circle winding test written separately in Python, half of them 0.2 m inside an edge; every point's s2sphere ancestor token at some allowed level is in the covering. Removing the edge-crossing test from the tool makes the coverings miss 218 of the 240 points

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `s2_covering_invariants`: every cell is within the level range asked for, and the reported coarsest and finest levels match the cells actually returned; the covered area is at least the region's own area, since a covering is a superset; no cell in a covering is an ancestor of another, so the set is not redundant; a wider level range or a larger budget never produces a covering that misses a point the tighter one held; and the count exceeds the budget only when a `COVERING_OVER_BUDGET` warning is present, so the one case where the contract bends is the one case that announces itself
