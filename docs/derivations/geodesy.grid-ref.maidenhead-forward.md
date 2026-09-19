<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Maidenhead locator (`geodesy.grid-ref.maidenhead-forward`)

## Method

The IARU Maidenhead locator. Shift longitude by 180° and latitude by 90°, then write pairs, longitude first: a field letter (18 × 18 fields of 20° × 10°, A–R), a square digit (10 × 10), a subsquare letter (24 × 24, a–x), and optionally an extended square digit and extended subsquare letter. The work is done in integer cells at the finest precision, so digits truncate exactly. A point on a grid line falls in the cell east or north of it; +90° and +180° fall in the last cell.

## Equations

- Finest cells: x = ⌊(λ + 180) × 2,880⌋, y = ⌊(φ + 90) × 5,760⌋ (1/2,880° of longitude by 1/5,760° of latitude).
- Pair k: (x div Bk) mod Dk and (y div Bk) mod Dk, with D = 18, 10, 24, 10, 24 and Bk the product of the D's after k.
- Cell sizes: 20° × 10°, 2° × 1°, 5′ × 2.5′, 30″ × 15″, 1.25″ × 0.625″.

## Symbols and units

φ latitude and λ longitude in degrees (WGS 84); locators of 2, 4, 6, 8, or 10 characters.

## Domain

Latitude −90° to 90° (beyond that, INVALID_INPUT), longitude −180° to 180°. A value within a millionth of a finest cell of a grid line counts as on the line, so decimal minutes typed as decimals (70.83333333333333 for 70°50′) land in the same cell whichever way the nearest double falls.

## Approximations

None. Integer cell arithmetic.

## Worked example

- sourcePublisher: Wikipedia contributors
- sourceTitle: Maidenhead Locator System
- sourceEdition: retrieved 2026-09-19
- sourceLocator: "W1AW, the American Radio Relay League's Hiram Percy Maxim Memorial Station in Newington, Connecticut, is found in grid locator FN31pr" (41.71463, −72.72713)
- independent: yes
- inputs: lat 41.71463, lon −72.72713, 6 characters
- outputs: FN31pr
- tolerance: exact string
- verifiedBy: golden vector v023, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-geodesy/tests/gridref_parity.rs`: `grid_references_match_python_libraries` compares 500 points (one in ten on a grid line or a pole) at 4 to 10 characters with the maidenhead 1.8.0 package, whose last-pair rounding and polar wrap are corrected by an exact rational check in the generator (7 of 2,000 locators)
- `tools/vectors/gen_gridref_diff.py`: regenerates that fixture
- `core/vectors/geodesy.grid-ref.maidenhead-forward.jsonl`: 23 vectors, including both poles and the antimeridian

## Invariants

- `core/crates/gp-geodesy/tests/gridref.rs` `grid_reference_invariants`: at every precision the cell holds the point, its center encodes back to the same code, and each finer cell lies inside the coarser one
