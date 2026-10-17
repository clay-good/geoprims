<!-- Derivation note (trust/correctness-program layer A). -->
# Coordinates in degrees and decimal minutes (`geodesy.parse.format-ddm`)

## Method

A latitude and longitude in decimal degrees are rewritten in degrees and decimal minutes. The value is rounded once, at the last place written, and a part that rounds up to 60 carries into the next. The operation is `geodesy.parse.format` with its notation fixed.

## Equations

- Minutes = 60 × the fraction of a degree; seconds = 60 × the fraction of a minute
- The last part is rounded to the decimal places asked for; 60 minutes or seconds carry
- Ground size of the last place: its angle × the meridional radius (latitude) or × the prime-vertical radius × cos φ (longitude), on WGS 84

## Symbols and units

Latitude and longitude in decimal degrees, north and east positive. Decimal places apply to the last part written (default 5). Hemisphere letters by default, or signs.

## Domain

Latitude from −90° to 90°; decimal places a whole number.

## Approximations

Rounding to the places written is the only loss. No datum is applied.

## Worked example

- sourcePublisher: Wikipedia contributors
- sourceTitle: Decimal degrees (worked example)
- sourceEdition: retrieved 2026-10-17
- sourceLocator: The US Capitol, 38°53′23″N 77°00′32″W, is 38.8897°, −77.0089° (38°53.383′N 77°00.533′W)
- independent: yes
- inputs: 38.8897222, −77.0088889 to 3 places
- outputs: 38°53.383'N 77°00.533'W
- tolerance: exact text
- verifiedBy: the `focused_parse_format_and_ellipsoid_operations_match_their_general_tools` integration test, which requires byte-for-byte parity with the stable `geodesy.parse.format` for the ddm notation
- verifiedOn: 2026-10-17

## Differential tests

- `tools/vectors/gen_geodesy_focused.py` `format_vectors`: 19 random positions written by exact decimal arithmetic in Python, half-up at the last place with carries, skipping exact ties.
- `core/vectors/geodesy.parse.format-ddm.jsonl`: those vectors, the page's example, and two refusal cases.
- The integration suite requires the whole result to equal `geodesy.parse.format` with the same notation chosen.

## Invariants

- `core/crates/gp-geodesy/tests/geodesy.rs` `focused_parse_format_and_ellipsoid_operations_match_their_general_tools`: the formatter equals the general `geodesy.parse.format` with its notation fixed, rejects a notation input, carries a part that rounds to 60 into the next place, and reads back to the resolution written.
- A part never prints as 60.
- More decimal places never move the position by more than half the last place of the shorter form.
