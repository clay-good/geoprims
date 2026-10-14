<!-- Derivation note (trust/correctness-program layer A). -->
# Precision of an MGRS reference (`geodesy.grid-ref.mgrs-precision`)

## Method

An MGRS reference with n digits per coordinate names a square 10^(5 − n) m across. Because MGRS truncates rather than rounds, dropping the last digit of each coordinate names the square, ten times larger, that contains the original. The operation validates the reference with the same decoder as `geodesy.grid-ref.mgrs-inverse`, counts its digits, and lists every coarser truncation.

## Equations

- Square size = 10^(5 − n) m for n digits per coordinate
- Coarser reference at k digits = the designator and square, then the first k digits of the easting and the first k of the northing

## Symbols and units

The reference as text, spaces allowed; the square size in meters, and in words.

## Domain

A valid MGRS or USNG reference with an even number of digits. An invalid one is refused with the decoder's reason.

## Approximations

None: precision is a property of the digits.

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency (GeoTrans), through the mgrs Python package
- sourceTitle: GeoTrans MGRS conversion
- sourceEdition: mgrs 1.5 (GeoTrans 3.x)
- sourceLocator: `MGRS().toMGRS(lat, lon, MGRSPrecision=k)` for k below the reference's own, which equals each truncation
- independent: yes
- inputs: 17T NE 86309 77770
- outputs: 1 m, 5 digits a side; coarser 17TNE86307777, 17TNE863777, 17TNE8677, 17TNE87, 17TNE
- tolerance: exact text
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-14

## Differential tests

- `tools/vectors/gen_mgrs_views.py` `precision_vectors`: 19 references written by GeoTrans at precisions 0 to 5 for random points, with every truncation checked against GeoTrans's own encoding at that precision.
- `core/vectors/geodesy.grid-ref.mgrs-precision.jsonl`: those vectors, the page's example, and three refusal cases.
- `core/crates/gp-geodesy/tests/geodesy.rs` `mgrs_zone_and_precision_match_the_mgrs_tools` requires each coarser square to contain the original.

## Invariants

- `core/crates/gp-geodesy/tests/geodesy_views_invariants.rs` `mgrs_views_invariants`: each coarser reference is the finer one's easting and northing cut to fewer digits, so its square contains the finer one; the square is ten times larger per digit dropped; and a 100 km reference has no coarser form.
