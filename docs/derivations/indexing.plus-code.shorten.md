<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Plus Code shortener (`indexing.plus-code.shorten`)

## Method

The Open Location Code shortening rule. Measure how far the reference point is from the code's center (the larger of the latitude and longitude differences, longitude across the antimeridian). Remove the first 8, 6, or 4 digits, the most that still recovers uniquely: a prefix can go when the reference lies within 0.3 of the resolution of the last pair removed. The code must be full and unpadded. One departure from the reference library: an 8-digit code is never shortened to a bare "+", which the project's own validityTests.csv marks invalid.

## Equations

- range = max(|φc − φr|, |Δλ|), with Δλ wrapped to ±180°.
- Remove 2(i + 1) digits for the largest i ∈ {3, 2, 1} with range < 0.3 × 20^(2 − i)°, keeping at least one digit.

## Symbols and units

φc, λc the code's center and φr, λr the reference point (degrees).

## Domain

Full codes without padding, and a reference point anywhere. Padded or short codes are refused with INVALID_INPUT. When the reference is too far away the code comes back whole.

## Approximations

None.

## Worked example

- sourcePublisher: Google (Open Location Code project)
- sourceTitle: Open Location Code test data, shortCodeTests.csv
- sourceEdition: open-location-code main, retrieved 2026-09-18
- sourceLocator: row 9C3W9QCJ+2VX, 51.3701125, −1.217765625 → +2VX
- independent: yes
- inputs: code 9C3W9QCJ+2VX, reference 51.3701125, −1.217765625
- outputs: +2VX
- tolerance: exact string
- verifiedBy: golden vector v007, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-indexing/tests/olc_official.rs`: every shortening row of shortCodeTests.csv through the library and through the tool
- `core/vectors/indexing.plus-code.shorten.jsonl`: 21 vectors, 15 of them from shortCodeTests.csv

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `plus_code_invariants`: a code shortened against a reference recovers to the full code at that reference, for every length from 8 to 15
