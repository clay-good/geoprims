<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# H3 grid ring (`indexing.h3.grid-ring`)

## Method

The hollow ring of cells exactly k grid steps from a cell (gridRing), from h3o, which follows H3 C's traversal.

## Equations

- ring(k) = disk(k) − disk(k − 1).
- Away from pentagons it holds 6k cells.

## Symbols and units

k grid distance, cells as H3 indexes.

## Domain

Any valid cell and k from 1 to 100. A ring that crosses pentagon distortion follows H3 C's safe traversal.

## Approximations

None: grid distance is exact in the index.

## Worked example

- sourcePublisher: Uber Technologies and the H3 contributors
- sourceTitle: H3 C library via h3-py
- sourceEdition: H3 C 4.4.1 (h3-py 4.4.2)
- sourceLocator: gridRing(892a8471487ffff, 1)
- independent: yes
- inputs: cell 892a8471487ffff, k 1
- outputs: 6 cells: 892a8471483ffff, 892a847148fffff, 892a8471497ffff, 892a84714b3ffff, 892a84714bbffff, 892a847334bffff
- tolerance: identical set of cells
- verifiedBy: automated differential test (below)
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-indexing/tests/h3_family_parity.rs`: 300 seeded cells at every resolution, one in eight a pentagon, against H3 C 4.4.1 via h3-py 4.4.2 through the public tools (rings of 1 to 4 steps, identical as sets)
- `tools/vectors/gen_h3_family_diff.py`: regenerates that fixture
- `core/vectors/indexing.h3.grid-ring.jsonl`: 20 vectors from H3 C, run through the core on every build

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `h3_family_invariants`: each ring is its disk minus the next smaller disk
