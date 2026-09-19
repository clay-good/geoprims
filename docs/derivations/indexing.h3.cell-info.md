<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# H3 cell inspector (`indexing.h3.cell-info`)

## Method

Decode an H3 index (hexadecimal, decimal, or 0x-prefixed) and report its center, its boundary, resolution, base cell, whether it is one of the 12 pentagons per resolution, whether its resolution is Class III (rotated about 19.1°), its area on the WGS 84 authalic sphere, and its decimal form, all from h3o.

## Equations

- Center and boundary: inverse gnomonic projection of the cell's face coordinates onto the icosahedron face, then to latitude and longitude.
- Area: the spherical area of the boundary polygon on the authalic sphere (R = 6,371.007 km), as the sum of its triangles' spherical excess.
- Class III: odd resolutions.

## Symbols and units

Cell as a 15-character H3 index, resolution 0 to 15, base cell 0 to 121, coordinates in degrees, area in km².

## Domain

Any valid H3 cell index. Invalid indexes (bad mode, reserved bits, or a digit of 7) are refused with INVALID_INPUT.

## Approximations

None beyond floating point. Areas agree with H3 C to 6e-15 relative at resolution 0, loosening about 7-fold per resolution to 2e-8 at resolution 15, where the spherical excess of a 1 m² cell runs out of digits in both.

## Worked example

- sourcePublisher: Uber Technologies and the H3 contributors
- sourceTitle: H3 C library via h3-py
- sourceEdition: H3 C 4.4.1 (h3-py 4.4.2)
- sourceLocator: cellToLatLng, getResolution, getBaseCellNumber, isResClassIII, and cellArea for 892a8471487ffff
- independent: yes
- inputs: cell 892a8471487ffff
- outputs: center 40.44486597844062, −79.98184690375207; resolution 9; base cell 21; Class III; 0.1053320799501 km²
- tolerance: 1e-12° for the center, 5e-8 relative for the area
- verifiedBy: automated differential test (below)
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-indexing/tests/h3_family_parity.rs`: 300 seeded cells at every resolution, one in eight a pentagon, against H3 C 4.4.1 via h3-py 4.4.2 through the public tools (center, resolution, base cell, pentagon and Class III flags, area, and every boundary vertex)
- `tools/vectors/gen_h3_family_diff.py`: regenerates that fixture
- `core/vectors/indexing.h3.cell-info.jsonl`: 23 vectors from H3 C, run through the core on every build

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `h3_family_invariants`: pentagon cells have five edges and vertexes and hexagons six, and each child of a cell names the cell as its parent
