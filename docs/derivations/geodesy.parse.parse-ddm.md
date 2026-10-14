<!-- Derivation note (trust/correctness-program layer A). -->
# Degrees and decimal minutes to decimal degrees (`geodesy.parse.parse-ddm`)

## Method

A position written in degrees and decimal minutes is read and returned in decimal degrees. The operation is the grammar of `geodesy.parse.coordinates` restricted to one notation: text that reads as any other notation is refused, with the notation it reads as named.

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

- sourcePublisher: geoprims
- sourceTitle: add-geodesy-suite specification
- sourceEdition: 2026
- sourceLocator: geodesy/coordinate-parsing, requirement "Parsing"; checked by hand arithmetic, not taken from a published table
- independent: no
- inputs: 38°41.250'N 9°25.500'W
- outputs: 38.6875°, −9.425°
- tolerance: 1e-12°
- verifiedBy: golden vector v001 and the `focused_parse_format_and_ellipsoid_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_geodesy_focused.py` `parse_vectors`: 19 random positions written three ways (a space, a comma, hemisphere first) and converted by exact fractions in Python.
- `core/vectors/geodesy.parse.parse-ddm.jsonl`: those vectors, the page's example, and four refusal cases, one for each other kind of text.
- The integration suite requires the whole result to equal `geodesy.parse.coordinates` for text in this notation.

## Invariants

- The sign follows the hemisphere letter.
- Text in another notation is refused, never reinterpreted.
- Formatting the result in the same notation returns the text's own digits.
