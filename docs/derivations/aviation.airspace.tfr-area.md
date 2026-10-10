<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# TFR and NOTAM area (`aviation.airspace.tfr-area`)

## Method

The tool draws the area a NOTAM describes in text. A circle is 72 points, each found by the geodesic direct problem on WGS 84 (Karney 2013) from the center at azimuths 360°, 355°, ... 5°, so the ring runs counterclockwise. The center is packed NOTAM coordinates (`393400N1224330W`, or minutes only), or a fix-radial-distance (`ABC012098.7`) carried from the navaid's entered position along radial + variation for the distance. A point list is used as entered, with a repeated closing point dropped. The area is Karney's geodesic polygon area of the ring. The bounds are the least and greatest latitude and longitude of the ring points. Floor and ceiling are normalized text (`SFC`, `3000 ft MSL`, `500 ft AGL`, `FL180`) and ride along as GeoJSON properties.

## Equations

- Packed coordinates: lat = DD + MM/60 + SS/3600 (or DD + MM/60), negative for S; lon = DDD + MM/60 + SS/3600, negative for W.
- Fix-radial-distance center: (lat_c, lon_c) = direct(navaid, azimuth = (radial + variation) mod 360, s = distance × 1,852 m).
- Circle points: P_k = direct(center, azimuth = 360 − 5k, s = radius), k = 0 … 71.
- Area = |geodesic polygon area of P_0 … P_71| (Karney), in square meters, shown in NM² (1 NM = 1,852 m).
- Bounds: south = min lat, north = max lat, west = min lon, east = max lon over the ring.
- GeoJSON ring: reversed when the planar shoelace sum Σ (lon_i·lat_{i+1} − lon_{i+1}·lat_i) is negative, so it runs counterclockwise, then closed on its first point (RFC 7946).

## Symbols and units

Latitude and longitude in degrees on WGS 84; radius and fix distance in nautical miles (1,852 m); area in NM²; variation in degrees, east positive.

## Domain

Radius above 0 and up to 500 NM. A point list has 3 to 200 points with at least 3 distinct ones and no repeated corner. Minutes and seconds must be under 60, latitude within ±90°, longitude within ±180°. An area whose west-to-east span exceeds 180° (one that crosses the antimeridian) is refused with OUT_OF_DOMAIN. A fix-radial-distance needs the navaid latitude, longitude, and variation.

## Approximations

The circle is a 72-sided geodesic polygon, so its edges cut inside the true circle: the area is about 0.1% below a true circle's (28.24 vs 28.27 NM² at 3 NM). Points are exact on WGS 84. The tool draws only what is entered; it does not read NOTAM times or know whether a TFR is active.

## Worked example

- sourcePublisher: geoprims (spec scenario; area and bounds computed by GeographicLib)
- sourceTitle: add-practitioner-essentials, aviation/instrument-procedures spec, "NOTAM and TFR geometry"
- sourceEdition: 2026
- sourceLocator: Scenario "TFR circle" (393400N1224330W, radius 3 NM, SFC to 3,000 ft MSL)
- independent: no
- inputs: center 393400N1224330W, radius 3 NM, floor SFC, ceiling 3000 FT MSL
- outputs: center 39.5666667°, −122.7250000°; area 28.24 NM² (28.238458882726746 by GeographicLib Planimeter on GeodSolve's 72 points); south 39.51662°, north 39.61671°, west −122.78966°, east −122.66034°; floor SFC, ceiling 3000 ft MSL
- tolerance: 1e-6 NM² for area, 1e-9° for bounds and center
- verifiedBy: golden vector v001
- verifiedOn: 2026-10-09

## Differential tests

- `tools/vectors/gen_tfr.py`: decodes the packed coordinates by hand, builds the 72-point circle with GeographicLib's GeodSolve (C++), and takes every area from GeographicLib's Planimeter, independent of the core's geographiclib-rs; covers a northern and a southern circle, a fix-radial-distance, a point list, and refusals
- `core/vectors/aviation.airspace.tfr-area.jsonl`: those vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/src/tfr.rs` `packed_and_frd_forms`: unit test of the parsers only; packed seconds and minutes forms decode to the right degrees and hemispheres, 60 minutes and too-short forms are refused, and fix-radial-distance strings split into radial and distance with radials of 360 or more refused. No test checks a geometric invariant of the drawn area.
- `core/crates/gp-aviation/src/tfr.rs` `altitude_limits`: SFC, feet MSL, feet AGL, and flight-level limits normalize as listed, and an unreadable limit is refused
