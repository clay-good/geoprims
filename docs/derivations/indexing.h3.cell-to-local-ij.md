<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# H3 local IJ for a cell (`indexing.h3.cell-to-local-ij`)

## Method

H3's cells sit on the twenty faces of an icosahedron, and a set of cells near one another can be unfolded onto a single plane with two axes, I and J. Around a chosen origin cell, every cell the unfolding reaches has one pair of whole-number coordinates on that plane, so the difference between two cells becomes a vector that can be added and subtracted. The coordinates mean nothing without the origin they were measured from, so the anchor is returned with them.

Where the unfolding cannot reach, or where one of the twelve pentagons breaks it, H3 refuses, and so does this: there is no correct answer to give.

## Equations

- The cell is resolved to its local IJK coordinates around the origin, and the redundant third axis removed: i = i − k, j = j − k, with k driven to zero.
- H3 C's `cellToLocalIj`, as implemented by h3o 0.11: the unfolding is a table of base-cell rotations rather than a closed form, so the coordinates are exact whole numbers rather than a computed approximation.

## Symbols and units

I and J are whole numbers on the unfolded plane, unbounded in principle and limited in practice by how far the unfolding reaches. The anchor is an H3 index. Both cells must be at one resolution.

## Domain

Cells near enough to the origin for the unfolding to reach, which covers many thousands of cells in ordinary use and stops at base-cell boundaries that cannot be unfolded together. A pentagon at either end is flagged with `PENTAGON_DISTORTION`.

## Approximations

None. The coordinates are whole numbers, so H3 C and this agree exactly rather than within a tolerance; there is nothing to round.

## Worked example

- sourcePublisher: Uber Technologies and the H3 contributors
- sourceTitle: H3 `cellToLocalIj`, through h3-py `cell_to_local_ij`
- sourceEdition: H3 C 4.4.1 via h3-py 4.4.2
- sourceLocator: `cell_to_local_ij` with the resolution 9 cell 892a8471487ffff as both origin and cell
- independent: yes
- inputs: origin 892a8471487ffff, cell 892a8471487ffff
- outputs: i 309, j 2469, anchor 892a8471487ffff
- tolerance: exact (whole numbers)
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-22

## Differential tests

- `tools/vectors/gen_h3_localij.py`: H3 C through h3-py over eleven origins at resolutions 3 to 13, each with the origin itself and a neighbour, every pair checked to round trip
- `core/vectors/indexing.h3.cell-to-local-ij.jsonl`: 22 vectors from that reference, run through the core on every build

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `local_ij_invariants`: around four origins, every cell of the disk has its own pair of coordinates, no two cells share a pair, the anchor comes back as the origin, the pair converts back to the cell it came from, and a cell across the globe is refused
