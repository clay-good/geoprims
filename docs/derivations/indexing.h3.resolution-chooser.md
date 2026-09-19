<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# H3 resolution chooser (`indexing.h3.resolution-chooser`)

## Method

Compare the target with H3's average hexagon area (or average edge length) at each of the 16 resolutions and pick the resolution nearest on a log scale, since each resolution is about 7 times smaller in area. Areas and edges come from h3o's tables, which are H3 C's. The full table, with the cell count at each resolution, comes back too.

## Equations

- Pick r minimizing |ln(A_r / A_target)| (or |ln(e_r / e_target)|).
- Cells at resolution r: 2 + 120 × 7^r (122 base cells, 12 pentagons at every resolution).

## Symbols and units

A_r the average hexagon area (km²) and e_r the average edge length (km) at resolution r (0–15). Targets are any area or length unit.

## Domain

A positive target area or edge length, one or the other. Targets outside the table's range get the nearest end (0 or 15).

## Approximations

Averages only: at one resolution, real cells range about 2 to 1 in area (smallest near the pentagons), so a chosen resolution fits the target on average, not cell by cell.

## Worked example

- sourcePublisher: Uber Technologies (H3 project)
- sourceTitle: H3 documentation, Tables of cell statistics across resolutions
- sourceEdition: h3geo.org, retrieved 2026-09-19
- sourceLocator: Average area in km², resolution 9: 0.105332513; cell count 4,842,432,842
- independent: yes
- inputs: target area 0.105332513 km²
- outputs: resolution 9, average area 0.105332513 km², 4,842,432,842 cells
- tolerance: 5e-10 km² (the table prints nine decimals)
- verifiedBy: golden vector v022, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-indexing/tests/dev_parity.rs`: `resolution_table_matches_h3_c` compares all 16 rows (area, edge, cell count) with H3 C 4.4.1 via h3-py 4.4.2
- `tools/vectors/gen_dev_diff.py`: regenerates that fixture
- `core/vectors/indexing.h3.resolution-chooser.jsonl`: 22 vectors across areas and edge lengths in several units

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `chooser_and_fill_invariants`: each resolution's own average area and edge choose that resolution, and resolutions never get coarser down the table
