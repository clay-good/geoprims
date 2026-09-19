<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# H3 grid path (`indexing.h3.grid-path`)

## Method

The grid distance between two cells and the line of cells joining them (gridDistance, gridPathCells), computed in local IJ coordinates by h3o as H3 C does. Where H3 C cannot draw the path (across pentagon distortion, or too far apart), the tool refuses in the same cases.

## Equations

- Both cells to local IJ coordinates about the origin cell.
- Cells along the line at each step are rounded to the nearest cell (cube rounding), giving distance + 1 cells.

## Symbols and units

Cells as H3 indexes; distance in grid steps.

## Domain

Two valid cells at the same resolution, near enough and not separated by pentagon distortion.

## Approximations

None: the same local IJ algorithm as H3 C.

## Worked example

- sourcePublisher: Uber Technologies and the H3 contributors
- sourceTitle: H3 C library via h3-py
- sourceEdition: H3 C 4.4.1 (h3-py 4.4.2)
- sourceLocator: gridPathCells(892a8471487ffff, 892a847148bffff)
- independent: yes
- inputs: from 892a8471487ffff to 892a847148bffff
- outputs: distance 2: 892a8471487ffff, 892a8471483ffff, 892a847148bffff
- tolerance: identical cells in order
- verifiedBy: automated differential test (below)
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-indexing/tests/h3_family_parity.rs`: 300 seeded cells at every resolution, one in eight a pentagon, against H3 C 4.4.1 via h3-py 4.4.2 through the public tools (paths to a cell up to 5 steps away, identical in order, and the 23 paths H3 C refuses also refused)
- `tools/vectors/gen_h3_family_diff.py`: regenerates that fixture
- `core/vectors/indexing.h3.grid-path.jsonl`: 20 vectors from H3 C, run through the core on every build

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `h3_family_invariants`: a path starts and ends at its cells, has distance + 1 cells, and steps only between neighbors
