<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# H3 uncompact (`indexing.h3.uncompact`)

## Method

Expand a mixed-resolution set of cells to one finer resolution (uncompactCells), the inverse of compact.

## Equations

- Each cell becomes its children at the target resolution; cells already at it stay.

## Symbols and units

A list of H3 cells and a target resolution.

## Domain

Valid cells no finer than the target resolution, within the shared list limits.

## Approximations

None.

## Worked example

- sourcePublisher: Uber Technologies and the H3 contributors
- sourceTitle: H3 C library via h3-py
- sourceEdition: H3 C 4.4.1 (h3-py 4.4.2)
- sourceLocator: uncompactCells([892a8471487ffff], 10)
- independent: yes
- inputs: cells [892a8471487ffff], resolution 10
- outputs: its 7 children
- tolerance: identical set
- verifiedBy: automated differential test (below)
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-indexing/tests/h3_family_parity.rs`: 300 seeded cells at every resolution, one in eight a pentagon, against H3 C 4.4.1 via h3-py 4.4.2 through the public tools (expanding each compacted set back, identical as sets)
- `tools/vectors/gen_h3_family_diff.py`: regenerates that fixture
- `core/vectors/indexing.h3.uncompact.jsonl`: 20 vectors from H3 C, run through the core on every build

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `h3_family_invariants`: uncompacting a cell gives exactly its children
