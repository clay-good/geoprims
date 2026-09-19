<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# H3 children (`indexing.h3.children`)

## Method

The finer cells inside a cell at a chosen resolution (cellToChildren), their count, and the center child (cellToCenterChild).

## Equations

- A hexagon has 7^d descendants d resolutions finer.
- A pentagon has 1 + 5(7^d − 1)/6, because its missing K-axis digit removes one subtree at each step.

## Symbols and units

Cells as H3 indexes, d resolutions finer.

## Domain

A valid cell and a target resolution no coarser than the cell's, within the shared list limits (output pages over long lists).

## Approximations

None: exact index arithmetic.

## Worked example

- sourcePublisher: Uber Technologies and the H3 contributors
- sourceTitle: H3 C library via h3-py
- sourceEdition: H3 C 4.4.1 (h3-py 4.4.2)
- sourceLocator: cellToChildren(892a8471487ffff, 10) and cellToCenterChild
- independent: yes
- inputs: cell 892a8471487ffff, resolution 10
- outputs: 7 children, center child 8a2a84714847fff
- tolerance: exact
- verifiedBy: automated differential test (below)
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-indexing/tests/h3_family_parity.rs`: 300 seeded cells at every resolution, one in eight a pentagon, against H3 C 4.4.1 via h3-py 4.4.2 through the public tools (children one or two resolutions finer, count and center child)
- `tools/vectors/gen_h3_family_diff.py`: regenerates that fixture
- `core/vectors/indexing.h3.children.jsonl`: 20 vectors from H3 C, run through the core on every build

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `h3_family_invariants`: hexagons have 7^d children and pentagons 1 + 5(7^d − 1)/6, and all name the cell as parent
