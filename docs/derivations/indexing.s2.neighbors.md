<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# S2 cells across the edges (`indexing.s2.neighbors`)

## Method

An S2 cell is a quadrilateral on a cube face, so it has four cells across its edges. Each is found by stepping one cell in i or j on the face's integer grid and rebuilding the id from the result. A step that leaves the face is the interesting case: the cube's faces meet at right angles and their grids do not line up, so the coordinates are carried onto the neighbouring face through S2's face-to-face rotation, and the answer says when that happened.

## Equations

- The cell's (i, j) at its level, from the Hilbert position.
- The four steps: (i ± 1, j) and (i, j ± 1).
- A step outside [0, 2^level) leaves the face: the (face, i, j) triple is taken through the cube's adjacency, which rotates and may swap the axes, then repacked.
- Each neighbour is rebuilt at the same level, so the four are always at the level asked for.

## Symbols and units

The cell and its neighbours are tokens. Level 0 to 30. The flag says how many of the four lie on another cube face, which is zero for most cells and non-zero along the twelve face edges.

## Domain

Any valid cell from level 0 to 30. At level 0 the six faces are neighbours of one another, which is the case where every step crosses an edge.

## Approximations

None. This is integer arithmetic on the grid and a fixed table of face adjacencies, so it agrees with an independent implementation exactly or not at all, in the same order.

## Worked example

- sourcePublisher: s2sphere contributors
- sourceTitle: s2sphere, a pure-Python implementation of the S2 geometry library
- sourceEdition: s2sphere 0.2.5
- sourceLocator: `CellId.from_token('8834f3dec').get_edge_neighbors()`, run for this note
- independent: yes
- inputs: cell 8834f3dec
- outputs: the four neighbours 8834f3de4, 8834f3df4, 8834f3d94, 8834f3dc4, in that order, none of them across a face edge
- tolerance: exact strings, in order
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-22

## Differential tests

- `tools/vectors/gen_s2.py`: 400 cases against s2sphere, whose edge neighbours are compared exactly and in the same order, including cells on the face edges where the rotation applies
- `core/crates/gp-indexing/tests/s2_parity.rs`: that fixture, run on every build
- `core/vectors/indexing.s2.neighbors.jsonl`: 22 vectors over every cube face, both poles, and the antimeridian

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `s2_invariants`: over six places and six levels, a cell has four neighbours, they are four different cells, each is at the cell's own level, and each of them counts the original among its own neighbours
