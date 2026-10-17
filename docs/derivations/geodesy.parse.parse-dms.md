<!-- Derivation note (trust/correctness-program layer A). -->
# Degrees, minutes, and seconds to decimal degrees (`geodesy.parse.parse-dms`)

## Method

A position written in degrees, minutes, and seconds is read and returned in decimal degrees. The operation is the grammar of `geodesy.parse.coordinates` restricted to one notation: text that reads as any other notation is refused, with the notation it reads as named.

## Equations

- Decimal degrees = degrees + minutes / 60 + seconds / 3,600
- South and west are negative

## Symbols and units

Degrees, minutes, and seconds as written, with symbols, spaces, or dashes between them and a hemisphere letter or a sign. Output in decimal degrees, north and east positive.

## Domain

Minutes and seconds under 60; latitude to 90° and longitude to 180°. A decimal pair, a grid reference, or the other sexagesimal notation is refused.

## Approximations

None: the conversion is exact arithmetic. No datum is applied.

## Worked example

- sourcePublisher: Wikipedia contributors
- sourceTitle: Decimal degrees (worked example)
- sourceEdition: retrieved 2026-10-17
- sourceLocator: The US Capitol, 38°53′23″N 77°00′32″W, is 38.8897°, −77.0089° (D + M/60 + S/3600)
- independent: yes
- inputs: 38°53'23"N 77°00'32"W
- outputs: 38.8897222°, −77.0088889° (the published rounding is 38.8897°, −77.0089°)
- tolerance: 5e-5° (the published rounding)
- verifiedBy: the `focused_parse_format_and_ellipsoid_operations_match_their_general_tools` integration test, which requires byte-for-byte parity with the stable `geodesy.parse.coordinates`, whose own published example (golden vector v008) is this same US Capitol conversion
- verifiedOn: 2026-10-17

## Differential tests

- `tools/vectors/gen_geodesy_focused.py` `parse_vectors`: 19 random positions written three ways (a space, a comma, hemisphere first) and converted by exact fractions in Python.
- `core/vectors/geodesy.parse.parse-dms.jsonl`: those vectors, the page's example, and four refusal cases, one for each other kind of text.
- The integration suite requires the whole result to equal `geodesy.parse.coordinates` for text in this notation.

## Invariants

- `core/crates/gp-geodesy/tests/geodesy.rs` `focused_parse_format_and_ellipsoid_operations_match_their_general_tools`: the reader agrees byte-for-byte with the any-notation `geodesy.parse.coordinates` on DMS text, refuses other notations by name, and a position formatted and read back returns to the resolution written.
- The sign follows the hemisphere letter.
- Text in another notation is refused, never reinterpreted.
