<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Maidenhead locator to position (`geodesy.grid-ref.maidenhead-inverse`)

## Method

Read the locator's pairs (case-insensitive) back into its cell: each pair adds its index times the cell size at that level to the south-west corner, starting from 180° W, 90° S. The result is the cell's bounds and center.

## Equations

- west = −180 + Σ ik × wk, south = −90 + Σ jk × hk.
- Cell sizes wk × hk: 20° × 10°, 2° × 1°, 5′ × 2.5′, 30″ × 15″, 1.25″ × 0.625″.
- Center: (south + h/2, west + w/2).

## Symbols and units

ik, jk the longitude and latitude index of pair k; w and h the cell's width and height in degrees.

## Domain

Locators of 2, 4, 6, 8, or 10 characters with each pair in range (fields A–R, squares 0–9, subsquares A–X in either case). Anything else is refused with INVALID_INPUT.

## Approximations

None. The bounds are exact sums of the cell sizes.

## Worked example

- sourcePublisher: Wikipedia contributors
- sourceTitle: Maidenhead Locator System
- sourceEdition: retrieved 2026-09-19
- sourceLocator: W1AW (41.71463, −72.72713) is in FN31pr
- independent: yes
- inputs: FN31pr
- outputs: the subsquare 41.708° to 41.750° N, 72.750° to 72.667° W, which holds W1AW
- tolerance: half a subsquare (1/48° of latitude, 1/24° of longitude) between its center and the station
- verifiedBy: golden vector v025, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-geodesy/tests/gridref_parity.rs`: `grid_references_match_python_libraries` compares 500 points (one in ten on a grid line or a pole) at 4 to 10 characters with the maidenhead 1.8.0 package, whose last-pair rounding and polar wrap are corrected by an exact rational check in the generator (7 of 2,000 locators)
- `tools/vectors/gen_gridref_diff.py`: regenerates that fixture
- `core/vectors/geodesy.grid-ref.maidenhead-inverse.jsonl`: 25 vectors, including lowercase and swapped-case locators and three refusals

## Invariants

- `core/crates/gp-geodesy/tests/gridref.rs` `grid_reference_invariants`: at every precision the cell holds the point, its center encodes back to the same code, and each finer cell lies inside the coarser one
