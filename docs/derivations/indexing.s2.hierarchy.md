<!-- Derivation note (trust/correctness-program layer A). -->
# S2 cell hierarchy (`indexing.s2.hierarchy`)

## Method

Every S2 cell below level 0 has one parent, and every cell above level 30 has four children ordered along the Hilbert curve. The ancestor at a level keeps the id's leading bits and sets the lowest of the rest. The operation uses the same calls as `indexing.s2.cell-info`.

## Equations

- Parent at level k: keep the face and the first 2k position bits, then set the next bit
- Children: the four cells whose ids extend the cell's by two bits, in Hilbert order

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
- sourceLocator: `parent()`, `parent(level)`, and `children()`
- independent: yes
- inputs: 89c25a3, ancestor at level 6
- outputs: the parent, the level 6 ancestor, and the four children
- tolerance: exact
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_s2_tile_views.py`: 20 cases each, from random cells at every level or random tiles at zooms 0 to 22, answered by s2sphere or by the tile definitions written out in Python rather than by the core's code.
- `core/vectors/indexing.s2.hierarchy.jsonl`: those vectors and refusal cases.
- `core/crates/gp-indexing/tests/indexing.rs` `focused_s2_and_tile_operations_match_the_general_tools` requires the same answers as the general S2 and tile tools.

## Invariants

- `core/crates/gp-indexing/tests/s2_tile_views_invariants.rs` `s2_views_invariants`: each child's parent is the cell, one level finer; a parent is one level coarser; stepping up parent by parent reaches the ancestor asked for; level 0 has no parent; and level 30 has no children.
