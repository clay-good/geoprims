<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# H3 cell from latitude and longitude (`indexing.h3.lat-lng-to-cell`)

## Method

The H3 v4 algorithm through h3o (a Rust implementation of H3): project the point to the icosahedron face, convert to IJK coordinates at the resolution, and encode the cell index. The cell center and area come from the same library.

## Equations

- Face selection by the nearest face center; gnomonic projection onto the face; IJK coordinates with Class II/III aperture-7 rotations per resolution.
- Area: the spherical area of the cell boundary on the authalic sphere (H3's cellAreaKm2).

## Symbols and units

lat, lon (degrees, WGS 84 treated as a sphere by H3); resolution 0-15; cell index as 15 hex digits.

## Domain

Every latitude and longitude; resolutions 0 to 15.

## Approximations

H3 treats the Earth as a sphere, as the standard defines. Cell areas are spherical.

## Worked example

- sourcePublisher: Uber Technologies and the H3 contributors
- sourceTitle: H3 C library via h3-py
- sourceEdition: H3 C 4.4.1 (h3-py 4.4.2)
- sourceLocator: latLngToCell(40.446111, -79.982222, 9)
- independent: yes
- inputs: lat 40.446111, lon -79.982222, resolution 9
- outputs: 892a8471487ffff
- tolerance: exact index; center 1e-9°
- verifiedBy: automated differential test (below)
- verifiedOn: 2026-09-18

## Differential tests

- `core/crates/gp-indexing/tests/h3_c_parity.rs`: 4,000 comparisons against H3 C 4.4.1 in CI (250 points × 16 resolutions); the full 100,000 × 16 run behind --ignored had 0 index mismatches

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `lat_lng_to_cell_invariants`: a cell's center maps back to the cell, the index encodes the resolution, and area shrinks about 7× per resolution
