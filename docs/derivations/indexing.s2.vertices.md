<!-- Derivation note (trust/correctness-program layer A). -->
# S2 cell vertices (`indexing.s2.vertices`)

## Method

An S2 cell is a quadrilateral on the unit sphere bounded by four great-circle arcs. The operation returns its four corners, from the same cell geometry as `indexing.s2.cell-info`, as latitude and longitude.

## Equations

- Corners from the cell's (u, v) bounds on its cube face, projected to the sphere with S2's quadratic transform
- Latitude and longitude from each unit vector

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
- sourceLocator: `Cell.get_vertex(k)` for k = 0 to 3
- independent: yes
- inputs: 47e66f
- outputs: the four corners counterclockwise
- tolerance: 1e-11°
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_s2_tile_views.py`: 20 cases each, from random cells at every level or random tiles at zooms 0 to 22, answered by s2sphere or by the tile definitions written out in Python rather than by the core's code.
- `core/vectors/indexing.s2.vertices.jsonl`: those vectors and refusal cases.
- `core/crates/gp-indexing/tests/indexing.rs` `focused_s2_and_tile_operations_match_the_general_tools` requires the same answers as the general S2 and tile tools.

## Invariants

- `core/crates/gp-indexing/tests/s2_tile_views_invariants.rs` `s2_views_invariants`: a cell has four corners, counterclockwise seen from outside the sphere, and each is also a corner of one of its neighbors.
