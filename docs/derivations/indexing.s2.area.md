<!-- Derivation note (trust/correctness-program layer A). -->
# S2 cell area (`indexing.s2.area`)

## Method

The cell's exact area is the spherical excess of its quadrilateral, scaled by the square of the Earth's mean radius, 6,371,008.8 m. The average is the sphere's area divided by the 6 × 4^level cells at the level. Both come from the same calls as `indexing.s2.cell-info`.

## Equations

- Area = Ω × R², Ω the cell's solid angle in steradians
- Average = 4πR² / (6 × 4^level)
- Ratio = area ÷ average

## Symbols and units

S2 cells as tokens or 64-bit decimal ids; tiles as zoom/x/y or quadkeys. Angles in degrees, areas in square meters.

## Domain

A valid S2 cell, or a tile whose column and row are within its zoom level. Invalid text is refused at its input with INVALID_INPUT.

## Approximations

None for indexes and tokens. Areas are on a sphere of the Earth's mean radius, not the ellipsoid.

## Worked example

- sourcePublisher: Sidewalk Labs (s2sphere), after Google's S2 Geometry library
- sourceTitle: s2sphere, a Python port of the S2 geometry library
- sourceEdition: s2sphere 0.2.5
- sourceLocator: `Cell.exact_area()` and `Cell.average_area()`, times R²
- independent: yes
- inputs: 6b12ae3fc
- outputs: its area, its level's average, and the ratio
- tolerance: 1e-8 relative, the indexing ceiling
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_s2_tile_views.py`: 20 cases each, from random cells at every level or random tiles at zooms 0 to 22, answered by s2sphere or by the tile definitions written out in Python rather than by the core's code.
- `core/vectors/indexing.s2.area.jsonl`: those vectors and refusal cases.
- `core/crates/gp-indexing/tests/indexing.rs` `focused_s2_and_tile_operations_match_the_general_tools` requires the same answers as the general S2 and tile tools.

## Invariants

- `core/crates/gp-indexing/tests/s2_tile_views_invariants.rs` `s2_views_invariants`: a cell's children's areas add up to its own to 1 part in 10⁹, the ratio to the level average stays between 0.5 and 1.5, and the six face cells add up to six times the level-0 average, the whole sphere.
