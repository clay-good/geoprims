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

- sourcePublisher: GeographicLib for Python (Karney) and pygeomag (an open-source Python implementation of the World Magnetic Model)
- sourceTitle: geographiclib 2.1, Geodesic.WGS84.Direct; pygeomag 1.1.0, GeoMag.calculate with the WMM2025 coefficients
- sourceEdition: geographiclib 2.1 and pygeomag 1.1.0, run 2026-10-10
- sourceLocator: tools/vectors/gen_radial_ref.py, first case: Direct(39.8, -104.7, 109, 23150) and calculate(glat=39.8, glon=-104.7, alt=0, time=2026.7233)
- independent: yes
- inputs: station 39.8°, −104.7°; radial 98°; distance 12.5 NM; variation 11° E; date 2026-09-22
- outputs: true course 109°; fix 39.7318374°, −104.4446687°; WMM2025 declination 7.3536° E; difference 3.6464°, with STATION_VARIATION_DIFFERS
- tolerance: 1e-9° for the fix and course; 1e-6° for the declination and the difference
- verifiedBy: golden vector v009, run by the core on every build
- verifiedOn: 2026-10-10

The same two libraries give 13 more cases, v010 through v022, at stations on six continents, in the Pacific, and in the Arctic, with one fix across the antimeridian.

## Differential tests

- `tools/vectors/gen_radial_ref.py`: the fix from GeographicLib for Python and the declination from pygeomag, neither of which the core uses, at 14 stations, radials, and dates across the WMM2025 window (fix within 1e-9°, declination within 1e-6°); no case sits within 0.05° of the 1° warning line
- `tools/vectors/gen_radial.py`: the fix from GeographicLib's GeodSolve (C++) at four stations; the declinations in these four are copied from geoprims' own declination tool, so they hold the answer steady rather than check the field
- `core/vectors/aviation.ifr.radial-fix.jsonl`: those vectors, a run without a date, and a date outside WMM2025, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/slice3.rs` `radial_fix_invariants`: at 25 stations and distances from 0 to 539 NM the true course is the radial plus the variation, the geodesic from the station to the fix has the entered length and leaves on the true course, moving 7° from the radial to the variation leaves the fix alone, and the warning appears exactly when the variation and WMM differ by more than 1°
