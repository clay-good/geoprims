<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# USNG to position (`geodesy.grid-ref.usng-inverse`)

## Method

Read the grid zone, the 100 km square letters, and the digits (spaces optional) into a UTM easting and northing, recover the northing's 2,000 km cycle from the latitude band, and invert UTM (or UPS in the polar caps). The result is the square's south-west corner and its center. A truncated reference without a grid zone ("NE 863 777") needs the zone given separately.

## Equations

- E = 100,000 × column + digits_E × 10^(5−d), N = 100,000 × row + 2,000,000 × k + digits_N × 10^(5−d), with k from the latitude band.
- Center: the corner plus half the square (10^(5−d) m) in each axis, then inverse UTM.

## Symbols and units

E easting and N northing in meters; d digits per axis (0 to 8); latitude and longitude in degrees; square size in meters.

## Domain

References with a valid zone, band, and square (UPS references in the polar caps too), with an even count of up to 16 digits; a local reference with a separate zone. Unknown letters, odd digit counts, and missing zones are refused with INVALID_INPUT.

## Approximations

NAD 83 is treated as WGS 84 (1 to 2 m in the conterminous US). The UTM inverse is Krüger's sixth-order series, accurate to 5 nm.

## Worked example

- sourcePublisher: Federal Geographic Data Committee
- sourceTitle: FGDC-STD-011-2001, United States National Grid
- sourceEdition: December 2001
- sourceLocator: Section 5.2.2 and table 1 (the Washington Monument at NAD 83 UTM zone 18 E 323,483.168 m, N 4,306,479.498 m); latitude and longitude of that corner from PROJ 9.3.0
- independent: yes
- inputs: 18S UJ 23483168 06479498
- outputs: corner 38.889467309501576, −77.0352402156242; square 0.001 m
- tolerance: 1e-10°
- verifiedBy: golden vector v022, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-geodesy/tests/gridref_parity.rs`: `usng_matches_geotrans_mgrs` writes the 2,000-point MGRS fixture (NGA GEOTRANS, digits from PROJ) as USNG and checks every non-polar row, 100 km to 1 m, through both USNG tools; corners agree within 2 cm
- `tools/vectors/gen_mgrs_diff.py`: regenerates that fixture
- `core/vectors/geodesy.grid-ref.usng-inverse.jsonl`: 22 vectors, including a local reference with its zone given separately

## Invariants

- `core/crates/gp-geodesy/tests/gridref.rs` `grid_reference_invariants`: the reference's square holds the point at 10 km to 1 m, and its center encodes back to it (or into the next zone, for a square cut by a zone edge)
