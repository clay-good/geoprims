<!-- Derivation note (trust/correctness-program layer A). -->
# Point from station and offset (`survey.cogo.point-from-station-offset`)

## Method

The operation computes plane coordinates from a station and signed offset on one straight survey line. The line begins at a known northing, easting, and station and is defined by either a direction or a second point. The target station determines the distance along the line. The offset is then applied along the right-hand normal; negative values place the point on the left.

## Equations

For line start S, unit tangent **t** = (cos α, sin α), right-hand unit normal **n** = (−sin α, cos α), start station s₀, target station s, and signed offset o:

- along = s − s₀
- foot = S + along × **t**
- point = foot + o × **n**

When an end point defines the line, α = atan2(Δeasting, Δnorthing).

## Symbols and units

Coordinates and offset use one compatible length unit: meters, international feet, or US survey feet. α is a north azimuth. Positive o is right while facing ahead on stationing; negative o is left. US station strings show two decimal places and metric station strings show three.

## Domain

Supply either a valid survey direction or both coordinates of a distinct line end. The target station may fall before the line start or beyond the supplied end because the calculation uses an infinite tangent. Coordinate and offset units must be compatible.

## Approximations

The construction is exact plane geometry subject to binary floating-point evaluation and display rounding. It does not apply a combined factor, trace a curve or spiral, or choose among segments of a route alignment.

## Worked example

- sourcePublisher: Autodesk
- sourceTitle: Civil 3D User Guide, To Create a Point From a Survey Baseline Using a Station and Offset
- sourceEdition: Civil 3D 2024 online help
- sourceLocator: help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-5D79BE01-65E7-4A68-A807-D05B7D8CA155.htm (BL TP 3 220 15.24 and BL TP 4 270 -15.24)
- independent: yes
- inputs: baseline from N 1,000, E 1,000 at station 0+000, azimuth 100°; station 220 with offset 15.24 m right, and station 270 with offset 15.24 m left
- outputs: N 946.7889, E 1,214.0113 (right) and N 968.1235, E 1,268.5445 (left), matching the page's points 3 and 4 to every printed digit
- tolerance: 1e-4 m against the page; 1e-9 relative against the Python evaluation in the golden vectors
- verifiedBy: golden vectors v024 and v025
- verifiedOn: 2026-10-19

## Differential tests

- `tools/vectors/gen_survey.py` independently constructs each point from tangent and normal vectors.
- `core/vectors/survey.cogo.point-from-station-offset.jsonl` contains 20 successful setout calculations and three refusal cases across all supported length units, plus the two Autodesk points (v024, v025).
- Cases alternate between direction-defined and two-point lines and cover positive, negative, and zero offsets.
- The integration suite requires exact parity with the setout mode of `survey.cogo.station-offset`.

## Invariants

- `core/crates/gp-survey/tests/cogo_views_invariants.rs` `station_offset_views_invariants`: at five line azimuths and three stations, the point less its foot is the entered signed offset along the right-hand normal; the point projects back to the entered station and offset; and a second point on the ray gives the same point as a direction.
