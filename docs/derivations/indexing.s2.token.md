<!-- Derivation note (trust/correctness-program layer A). -->
# S2 token and cell id (`indexing.s2.token`)

## Method

An S2 cell id is a 64-bit integer; its token is the same integer in hexadecimal with the trailing zeros dropped. The operation reads either form and writes both, with the level and cube face read from the id's bits, using the same reader as `indexing.s2.cell-info`.

## Equations

- Token = the id in lowercase hexadecimal, trailing zeros removed
- Level = 30 − (number of trailing zero bits − 1) / 2
- Face = the id's top 3 bits

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
- sourceLocator: `CellId.from_token`, `id()`, `level()`, and `face()`
- independent: yes
- inputs: 8085809c
- outputs: cell id 9260949616260546560, level 13, face 4
- tolerance: exact
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_s2_tile_views.py`: 20 cases each, from random cells at every level or random tiles at zooms 0 to 22, answered by s2sphere or by the tile definitions written out in Python rather than by the core's code.
- `core/vectors/indexing.s2.token.jsonl`: those vectors and refusal cases.
- `core/crates/gp-indexing/tests/indexing.rs` `focused_s2_and_tile_operations_match_the_general_tools` requires the same answers as the general S2 and tile tools.

## Invariants

- `core/crates/gp-indexing/tests/s2_tile_views_invariants.rs` `s2_views_invariants`: a token's decimal id is 17 or more digits, and reading that id back gives the token.
