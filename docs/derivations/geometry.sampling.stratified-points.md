<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Stratified random sample points (`geometry.sampling.stratified-points`)

## Method

Square cells are the strata, and each gets k random points in its part of the polygon. One random stream runs through the strata in a fixed order, so the seed reproduces every point.

1. **Plane and triangles.** As in `geometry.sampling.random-points` (steps 1 to 3 of its note).
2. **Cells.** The ground cell side s is multiplied by the zone's point scale at the outline's mean corner. Cells start at the lower-left corner of the outline's bounding box, with as many columns and rows as cover it.
3. **Pieces.** Each triangle is listed under every cell its bounding box reaches. A cell's part of the polygon is each listed triangle clipped to the cell (Sutherland-Hodgman against the cell's four sides: west, east, south, north), then fanned into triangles from the clipped polygon's first corner. Fan triangles with no area are dropped.
4. **Order.** Strata are taken row by row, south to north, with cells west to east, and labeled r(row)c(column) from 1. A cell with no part of the polygon is passed over. One with under 1 m² on the ground (k² plane m²) is skipped and listed.
5. **Points.** In each stratum, k points are drawn by the random-points draw: two draws each, the first picking a piece by area and serving, rescaled, as r₁. All strata share one xoshiro256** stream.

## Equations

- Plane cell side s_p = s · k(φ₀, λ₀).
- Point in a triangle: P = (1 − √r₁)A + √r₁(1 − r₂)B + √r₁r₂C (Osada et al. 2002 §4.2).
- Expected points, checked before any are made: min(plane area / s_p², cells) × k.

## Symbols and units

- s: cell side in m.
- k: points per stratum, 1 to 1,000.
- Seed: as in the random-points note.
- Points: in degrees, each with its stratum label.

## Domain

- At most 50,000 cells over the bounding box and 100,000 points expected, or `LIMIT_EXCEEDED`.
- The polygon spans at most 500 km and has at most 2,000 corners.

## Approximations

Points are uniform within each stratum by plane area, as in the random-points note. Every stratum gets k points whatever its share of the polygon, as EPA QA/G-5S ch. 6 describes for equal allocation. Estimates must weigh strata by area.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: A field cut so its east column of cells holds a 2 mm sliver
- sourceEdition: 2026-10-20
- sourceLocator: The cell side is (plane width − 2 mm)/2 over the scale, so the third column holds under 1 m² of the field
- independent: no (no published stratified layout is reproduced)
- inputs: the 340 m × 333 m field, that cell side, seed 9
- outputs: every third-column cell listed as skipped, with no points
- tolerance: exact
- verifiedBy: `stratified_points_scenarios`
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_grid_designs.py stratified`: the method above reimplemented in Python with PROJ's transverse Mercator and scale factor, sharing the stream and ear clipping of `gen_random_points.py`. It covers 20 cases over five polygons (holes, the southern hemisphere, the equator) with 1 to 3 points per stratum. Counts and strata agree exactly, and the first 12 points within 1e-9° with their stratum labels.
- `core/vectors/geometry.sampling.stratified-points.jsonl`: those vectors and the too-many-cells refusal, run through the core on every build.

## Invariants

- `core/crates/gp-geometry/tests/sampling.rs` `stratified_points_scenarios`:
  - Every stratum has exactly k points, and the count is k times the strata.
  - A cell holding under 1 m² is skipped, listed, and gets no points.
