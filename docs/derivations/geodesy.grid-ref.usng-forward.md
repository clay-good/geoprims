<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# USNG for a point (`geodesy.grid-ref.usng-forward`)

## Method

The US National Grid is MGRS on NAD 83 (taken as WGS 84, within a meter or two) written with spaces: grid zone designation, 100 km square letters, then easting and northing digits. The reference comes from the MGRS encoder (UTM, with the Norway and Svalbard zone exceptions, and UPS in the polar caps), truncated to the chosen precision, never rounded.

## Equations

- Grid zone: UTM zone ⌊(λ + 180)/6⌋ + 1 (with the exceptions) and latitude band letter.
- 100 km square: column and row letters from the easting and northing (the AA lettering scheme).
- Digits: ⌊E mod 100,000 / 10^(5−d)⌋ and ⌊N mod 100,000 / 10^(5−d)⌋ for d digits per axis.

## Symbols and units

φ latitude and λ longitude in degrees; E easting and N northing in meters; precision grid-zone, 100km to 1m, and 0.1m to 0.001m.

## Domain

Latitude −90° to 90°, longitude −180° to 180°. North of 84° N and south of 80° S the reference is UPS, as in MGRS (Z AB 96454 52981 at 85° N, 10° E); the USNG standard itself covers only the UTM area.

## Approximations

NAD 83 is treated as WGS 84: the two differ by about 1 to 2 m in the conterminous US, below USNG's usual 1 m to 10 m precision but not below 0.1 m. The UTM projection is Krüger's sixth-order series, accurate to 5 nm.

## Worked example

- sourcePublisher: Federal Geographic Data Committee
- sourceTitle: FGDC-STD-011-2001, United States National Grid
- sourceEdition: December 2001
- sourceLocator: Section 5.2.2 and table 1 (the Washington Monument, NAD 83 UTM zone 18 E 323,483.168 m, N 4,306,479.498 m = 18S UJ 23483 06479, truncated to 18S UJ 2348 0647, 18S UJ 234 064, and 18S UJ 23 06); its latitude and longitude from PROJ 9.3.0
- independent: yes
- inputs: 38.889467309501576, −77.0352402156242 at 1 m, 10 m, 100 m, and 1 km
- outputs: 18S UJ 23483 06479, 18S UJ 2348 0647, 18S UJ 234 064, 18S UJ 23 06
- tolerance: identical strings
- verifiedBy: golden vectors v020 to v023, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-geodesy/tests/gridref_parity.rs`: `usng_matches_geotrans_mgrs` writes the 2,000-point MGRS fixture (NGA GEOTRANS, digits from PROJ) as USNG and checks every non-polar row, 100 km to 1 m, through both USNG tools; corners agree within 2 cm
- `tools/vectors/gen_mgrs_diff.py`: regenerates that fixture
- `core/vectors/geodesy.grid-ref.usng-forward.jsonl`: 23 vectors from GeographicLib's GeoConvert and the FGDC standard

## Invariants

- `core/crates/gp-geodesy/tests/gridref.rs` `grid_reference_invariants`: the reference's square holds the point at 10 km to 1 m, and its center encodes back to it (or into the next zone, for a square cut by a zone edge)
