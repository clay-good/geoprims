<!-- Derivation note (trust/correctness-program layer A). -->
# XYZ and TMS tile numbering (`indexing.tile.xyz-tms`)

## Method

The XYZ scheme of web maps and the TMS scheme of MBTiles number the same tiles, but count rows from opposite edges of the map. The conversion flips the row within the zoom level and leaves the zoom and column alone.

## Equations

- y_TMS = 2^z − 1 − y_XYZ, and back the same way

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
- sourceLocator: the XYZ and TMS row definitions
- independent: yes
- inputs: 14/4551/6176 in XYZ
- outputs: TMS 14/4551/10207
- tolerance: exact
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_s2_tile_views.py`: 20 cases each, from random cells at every level or random tiles at zooms 0 to 22, answered by s2sphere or by the tile definitions written out in Python rather than by the core's code.
- `core/vectors/indexing.tile.xyz-tms.jsonl`: those vectors and refusal cases.
- `core/crates/gp-indexing/tests/indexing.rs` `focused_s2_and_tile_operations_match_the_general_tools` requires the same answers as the general S2 and tile tools.

## Invariants

- Converting twice returns the tile.
- The column and zoom never change.
- At zoom 0 both schemes are 0/0/0.
