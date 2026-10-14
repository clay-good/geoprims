<!-- Derivation note (trust/correctness-program layer A). -->
# Map tile to quadkey and back (`indexing.tile.tile-quadkey`)

## Method

A quadkey writes a tile's position one zoom level at a time: each digit says which quarter of the previous tile it is in. The digit at level i is 2 × (bit i of y) + (bit i of x), most significant first. The operation shares its quadkey function with `indexing.tile.bounds`.

## Equations

- digit_i = 2·y_i + x_i for i from z − 1 down to 0
- The quadkey's length is the zoom

## Symbols and units

S2 cells as tokens or 64-bit decimal ids; tiles as zoom/x/y or quadkeys. Angles in degrees, areas in square meters.

## Domain

A valid S2 cell, or a tile whose column and row are within its zoom level. Invalid text is refused at its input with INVALID_INPUT.

## Approximations

None for indexes and tokens. Areas are on a sphere of the Earth's mean radius, not the ellipsoid.

## Worked example

- sourcePublisher: OpenStreetMap wiki and Microsoft
- sourceTitle: Slippy map tilenames; Bing Maps Tile System (quadkeys)
- sourceEdition: as published
- sourceLocator: the Bing Maps Tile System quadkey definition
- independent: yes
- inputs: 13/2275/3088
- outputs: its 13-digit quadkey
- tolerance: exact
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_s2_tile_views.py`: 20 cases each, from random cells at every level or random tiles at zooms 0 to 22, answered by s2sphere or by the tile definitions written out in Python rather than by the core's code.
- `core/vectors/indexing.tile.tile-quadkey.jsonl`: those vectors and refusal cases.
- `core/crates/gp-indexing/tests/indexing.rs` `focused_s2_and_tile_operations_match_the_general_tools` requires the same answers as the general S2 and tile tools.

## Invariants

- `core/crates/gp-indexing/tests/s2_tile_views_invariants.rs` `tile_views_invariants`: a quadkey has one digit per zoom level and begins with its parent tile's, reading it back gives the tile, and a TMS tile gives the same quadkey as its XYZ twin, since quadkeys count rows from the north.
