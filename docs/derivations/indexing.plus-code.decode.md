<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Plus Code decoder (`indexing.plus-code.decode`)

## Method

Validate the code (alphabet, separator position, padding), then read the digit pairs and grid digits back into the south-west corner and the cell size, as the Open Location Code reference does. A short code (digits removed before the '+') needs a reference point: the missing prefix is taken from the reference's own code, then moved by one cell of that size if that brings the result nearer the reference (the reference's recoverNearest).

## Equations

- South-west corner: sum of digit value × place resolution, less 90° (latitude) and 180° (longitude).
- Cell: pairs give 20^(2 − i)° squares; grid digits divide by 5 in latitude and 4 in longitude.
- Recovery: if the decoded center is more than half the removed resolution from the reference, move it one resolution toward the reference (never past a pole).

## Symbols and units

Bounds and center in degrees (WGS 84). The reference point is in degrees.

## Domain

Full codes of 2 to 15 digits (with 0 padding up to the separator for short lengths), and short codes with a reference point. Invalid codes are refused with INVALID_INPUT; a short code without a reference asks for one.

## Approximations

None. Digits are read as integers and scaled once to degrees, as in the reference implementations.

## Worked example

- sourcePublisher: Google (Open Location Code project)
- sourceTitle: Open Location Code test data, decoding.csv
- sourceEdition: open-location-code main, retrieved 2026-09-18
- sourceLocator: row 7FG49Q00+, length 6: south 20.35, west 2.75, north 20.4, east 2.8
- independent: yes
- inputs: code 7FG49Q00+
- outputs: south 20.35°, west 2.75°, north 20.4°, east 2.8°
- tolerance: 1e-9°
- verifiedBy: golden vector v008, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-indexing/tests/olc_official.rs`: every row of decoding.csv and validityTests.csv, plus the recovery rows of shortCodeTests.csv, through the library and through the tool
- `core/vectors/indexing.plus-code.decode.jsonl`: 49 vectors from decoding.csv, encoding.csv, and the recovery rule

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `plus_code_invariants`: the decoded cell holds the encoded point, and a shortened code recovers to the full code near its reference
