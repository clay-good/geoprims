<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Fix from a radial and distance (`aviation.ifr.radial-fix`)

## Method

A VOR radial is magnetic, referenced to the station's own published variation rather than today's magnetic field. The tool adds that variation to the radial to get the true course from the station, then solves the geodesic direct problem on WGS 84 (Karney 2013) from the station along that course for the ground distance. With a date, it also evaluates the WMM2025 declination at the station at sea level and reports the difference from the station's variation, warning (STATION_VARIATION_DIFFERS) when it exceeds 1°. The fix always uses the station's variation.

## Equations

- True course TC = (radial + variation) mod 360
- (fix_lat, fix_lon) = direct(station lat, station lon, azimuth TC, distance s)
- With a date: D = WMM2025 declination at the station, height 0; difference = variation − D; warn when |difference| > 1°

## Symbols and units

Station and fix latitude and longitude in degrees on WGS 84; radial, variation, and true course in degrees (variation east positive); distance over the ground, entered in NM (1,852 m); date as YYYY-MM-DD.

## Domain

Variation within ±60°; distance from 0 to 1,000 km (the message says 540 NM). A date outside the WMM2025 window gives OUT_OF_DOMAIN; leaving the date out skips the comparison.

## Approximations

The fix is exact geometry on WGS 84 for the distance entered. The distance must be over the ground: a DME reading is slant range and should be converted first. The WMM comparison is at sea level and only flags a gap; it never changes the fix.

## Worked example

- sourcePublisher: geoprims (spec scenario; fix by GeographicLib GeodSolve, declination by geoprims' own WMM2025 tool)
- sourceTitle: add-practitioner-essentials, aviation/instrument-procedures spec, "Station magnetic variation"
- sourceEdition: 2026
- sourceLocator: Scenario "Variation mismatch" (the fix uses the station's 11° E and warns about the difference from WMM); tool example at 39.8°, −104.7°, radial 098, 12.5 NM, date 2026-09-22
- independent: no
- inputs: station 39.8°, −104.7°; radial 98°; distance 12.5 NM; variation 11° E; date 2026-09-22
- outputs: true course 109°; fix 39.7318374°, −104.4446687°; WMM2025 declination 7.35° E; difference 3.65°, with STATION_VARIATION_DIFFERS
- tolerance: 1e-9° for the fix, course, and declination
- verifiedBy: golden vector v001
- verifiedOn: 2026-10-09

## Differential tests

- `tools/vectors/gen_radial.py`: the fix from GeographicLib's GeodSolve (C++), independent of the core's geographiclib-rs, at four stations and radials (within 1e-9°); the WMM2025 declinations in it are copied from geoprims' own `geodesy.magnetic.declination` tool, so they are not an independent check of the field
- `core/vectors/aviation.ifr.radial-fix.jsonl`: those vectors, a run without a date, and a date outside WMM2025, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `golden_vectors`: no test checks an invariant of this tool; this is the closest test that exercises it, and it runs every vector in the tool's file, including the warning code and the refused date, through the core and fails on any value outside its tolerance
