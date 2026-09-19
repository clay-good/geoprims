<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# H3 parent (`indexing.h3.parent`)

## Method

The coarser cell containing a cell at a chosen resolution (cellToParent), by zeroing the finer digits, and the cell's position among that parent's descendants at its own resolution (cellToChildPos).

## Equations

- Parent: the index with digits finer than the target resolution set to 7 (unused) and the resolution field lowered.
- Child position: the cell's rank among the parent's descendants in index order.

## Symbols and units

Cells as H3 indexes, resolutions 0 to 15.

## Domain

A valid cell and a target resolution no finer than the cell's.

## Approximations

None: exact index arithmetic.

## Worked example

- sourcePublisher: Uber Technologies and the H3 contributors
- sourceTitle: H3 C library via h3-py
- sourceEdition: H3 C 4.4.1 (h3-py 4.4.2)
- sourceLocator: cellToParent(892a8471487ffff, 5) and cellToChildPos(892a8471487ffff, 5)
- independent: yes
- inputs: cell 892a8471487ffff, resolution 5
- outputs: parent 852a8473fffffff, child position 911
- tolerance: exact
- verifiedBy: automated differential test (below)
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-indexing/tests/h3_family_parity.rs`: 300 seeded cells at every resolution, one in eight a pentagon, against H3 C 4.4.1 via h3-py 4.4.2 through the public tools (parents at random coarser resolutions, with child positions)
- `tools/vectors/gen_h3_family_diff.py`: regenerates that fixture
- `core/vectors/indexing.h3.parent.jsonl`: 20 vectors from H3 C, run through the core on every build

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `h3_family_invariants`: every child of a cell names the cell as its parent
