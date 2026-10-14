<!-- Derivation note (trust/correctness-program layer A). -->
# Coordinates in degrees, minutes, and seconds (`geodesy.parse.format-dms`)

## Method

A latitude and longitude in decimal degrees are rewritten in degrees, minutes, and seconds. The value is rounded once, at the last place written, and a part that rounds up to 60 carries into the next. The operation is `geodesy.parse.format` with its notation fixed.

## Equations

- Minutes = 60 × the fraction of a degree; seconds = 60 × the fraction of a minute
- The last part is rounded to the decimal places asked for; 60 minutes or seconds carry
- Ground size of the last place: its angle × the meridional radius (latitude) or × the prime-vertical radius × cos φ (longitude), on WGS 84

## Symbols and units

Latitude and longitude in decimal degrees, north and east positive. Decimal places apply to the last part written (default 3). Hemisphere letters by default, or signs.

## Domain

Latitude from −90° to 90°; decimal places a whole number.

## Approximations

Rounding to the places written is the only loss. No datum is applied.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: add-geodesy-suite specification
- sourceEdition: 2026
- sourceLocator: geodesy/coordinate-parsing, requirement "Formatting"; checked by hand arithmetic, not taken from a published table
- independent: no
- inputs: 48.858370, 2.294481 to 1 place
- outputs: 48°51'30.1"N 2°17'40.1"E
- tolerance: exact text
- verifiedBy: golden vector v001 and the `focused_parse_format_and_ellipsoid_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_geodesy_focused.py` `format_vectors`: 19 random positions written by exact decimal arithmetic in Python, half-up at the last place with carries, skipping exact ties.
- `core/vectors/geodesy.parse.format-dms.jsonl`: those vectors, the page's example, and two refusal cases.
- The integration suite requires the whole result to equal `geodesy.parse.format` with the same notation chosen.

## Invariants

- Reading the text back returns the position to within the ground size reported.
- A part never prints as 60.
- More decimal places never move the position by more than half the last place of the shorter form.
