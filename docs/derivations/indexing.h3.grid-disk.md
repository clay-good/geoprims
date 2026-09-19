<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# H3 grid disk (`indexing.h3.grid-disk`)

## Method

All H3 cells within k grid steps of a cell, the center first, from h3o's gridDisk. Near a pentagon the fast hexagonal spiral would skip or repeat cells, so it falls back to the safe breadth-first traversal (as H3 C does).

## Equations

- For a disk with no pentagon inside: count = 1 + 3k(k + 1).
- Distortion near pentagons: the disk has fewer cells, since a pentagon has five neighbors.

## Symbols and units

k grid distance (0–100), cells as 15-character H3 indexes. The output count is a plain number.

## Domain

Any valid H3 cell, resolutions 0 to 15, and k from 0 to 100. Output is capped by the shared result limits (`output.maxItems` pages long lists).

## Approximations

None. Grid distance is exact in the H3 index. Cell order after the center follows h3o, so compare disks as sets.

## Worked example

- sourcePublisher: Uber Technologies and the H3 contributors
- sourceTitle: H3 C library via h3-py
- sourceEdition: H3 C 4.4.1 (h3-py 4.4.2)
- sourceLocator: gridDisk(85283473fffffff, 1)
- independent: yes
- inputs: cell 85283473fffffff, k 1
- outputs: 7 cells: 85283473fffffff, 85283447fffffff, 8528347bfffffff, 85283463fffffff, 85283477fffffff, 8528340ffffffff, 8528340bfffffff
- tolerance: identical set of cells
- verifiedBy: automated differential test (below)
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-indexing/tests/dev_parity.rs`: 250 disks (k 0 to 6, every resolution, one in ten centered on a pentagon) identical as sets to H3 C 4.4.1 via h3-py 4.4.2
- `tools/vectors/gen_dev_diff.py`: regenerates that fixture
- `core/vectors/indexing.h3.grid-disk.jsonl`: 22 vectors from H3 C, including five pentagons

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `grid_disk_invariants`: away from pentagons a disk holds 1 + 3k(k + 1) cells with the center first, disks nest (k − 1 inside k), and a pentagon's first ring has five cells
