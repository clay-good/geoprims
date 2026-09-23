<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# H3 cell at local IJ (`indexing.h3.local-ij-to-cell`)

## Method

The other half of the unfolding: given the origin the coordinates were measured from, fold the plane back onto the icosahedron and name the cell that lands at I, J. Having moved by a vector in local coordinates, this says which cell you arrived at.

A pair that no cell occupies — too far from the origin for the unfolding to reach, or across a pentagon where it breaks — is refused rather than answered with the nearest thing.

## Equations

- The pair is taken to local IJK around the anchor (k = 0), and folded back through the base-cell rotation the origin implies.
- H3 C's `localIjToCell`, as implemented by h3o 0.11. It is the exact inverse of `cellToLocalIj` wherever both are defined.

## Symbols and units

I and J are whole numbers and are refused if given with a fractional part, since a fraction names no cell. The origin is an H3 index, and the cell comes back at the origin's resolution. Latitude and longitude of the cell's centre are returned in degrees.

## Domain

Pairs within the unfolding's reach around the given origin. The same pair around a different origin is a different cell, so the two always travel together.

## Approximations

None in the cell: whole numbers in, an exact index out. The centre's latitude and longitude carry the ordinary floating-point rounding of H3's own cell-centre calculation.

## Worked example

- sourcePublisher: Uber Technologies and the H3 contributors
- sourceTitle: H3 `localIjToCell`, through h3-py `local_ij_to_cell`
- sourceEdition: H3 C 4.4.1 via h3-py 4.4.2
- sourceLocator: `local_ij_to_cell` at i 309, j 2469 around the resolution 9 origin 892a8471487ffff
- independent: yes
- inputs: origin 892a8471487ffff, i 309, j 2469
- outputs: cell 892a8471487ffff, latitude 40.44486597844062°, longitude −79.98184690375207°
- tolerance: exact on the cell; 1e-9 degrees on the centre
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-22

## Differential tests

- `tools/vectors/gen_h3_localij.py`: H3 C through h3-py over eleven origins at resolutions 3 to 13, every pair checked to round trip back to the cell it came from
- `core/vectors/indexing.h3.local-ij-to-cell.jsonl`: 22 vectors from that reference, run through the core on every build

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `local_ij_invariants`: around four origins, every cell of the disk converts to a pair and back to itself, no two cells share a pair, and a pair out of the unfolding's reach is refused
