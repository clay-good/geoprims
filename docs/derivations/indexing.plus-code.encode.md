<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Plus Code encoder (`indexing.plus-code.encode`)

## Method

The Open Location Code algorithm. Shift latitude and longitude to positive ranges (φ + 90, λ + 180), then write base-20 digit pairs (alphabet `23456789CFGHJMPQRVWX`) for the first 10 characters, latitude then longitude, each pair 20 times finer. Past 10 characters each digit subdivides the cell into a grid of 5 rows (latitude) by 4 columns (longitude). A '+' follows the eighth digit, and codes shorter than 8 digits are padded with 0.

## Equations

- Pair resolution for pair i (1–5): 20^(2 − i) degrees, so 20°, 1°, 0.05°, 0.0025°, 0.000125°.
- Grid digit j past 10: cell height 0.000125° / 5^j and width 0.000125° / 4^j.
- Latitude 90 is moved down by one cell height so the northernmost cell is valid.

## Symbols and units

φ latitude and λ longitude (degrees, WGS 84), n the code length (2, 4, 6, 8, or 10 to 15). Bounds are in degrees; cell sizes in meters on the mean-radius sphere.

## Domain

Latitude −90° to 90° and longitude any value (wrapped into −180° to 180°). Lengths of 2, 4, 6, 8, or 10 to 15 digits; lengths 9 and 16 or more, and latitudes past ±90°, are refused (the reference library would clip them silently).

## Approximations

One rounding, as in the reference implementations: shifted degrees are scaled by 8,000 × 5^5 (latitude) and 8,000 × 4^5 (longitude), rounded to 6 decimals, and floored to integers. All digits then come from integer arithmetic, so codes match the reference exactly.

## Worked example

- sourcePublisher: Google (Open Location Code project)
- sourceTitle: Open Location Code test data, encoding.csv
- sourceEdition: open-location-code main, retrieved 2026-09-18
- sourceLocator: row 20.3700625, 2.7821875, length 10 → 7FG49QCJ+2V
- independent: yes
- inputs: lat 20.3700625, lon 2.7821875, length 10
- outputs: 7FG49QCJ+2V
- tolerance: exact string
- verifiedBy: golden vector v002, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-indexing/tests/olc_official.rs`: every row of the official encoding.csv through the library, and through the tool (`official_cases_through_the_tools`), which must refuse the rows the reference clips
- `core/vectors/indexing.plus-code.encode.jsonl`: 35 vectors from encoding.csv

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `plus_code_invariants`: the cell holds the point, and encoding and decoding agree on the cell at every length
