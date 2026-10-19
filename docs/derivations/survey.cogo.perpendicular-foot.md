<!-- Derivation note (trust/correctness-program layer A). -->
# Perpendicular foot and station (`survey.cogo.perpendicular-foot`)

## Method

The operation projects a plane-coordinate point onto an infinite straight survey line. The line begins at a known northing and easting and is defined by either a direction or a second point. The projection supplies the along-line distance, which is added to the starting station, and the signed perpendicular offset. Positive offset is right while looking in the direction of increasing station.

## Equations

For start point S, observed point P, unit tangent **t** = (cos α, sin α), and right-hand unit normal **n** = (−sin α, cos α):

- along = (P − S) · **t**
- offset = (P − S) · **n**
- station = start station + along
- foot = S + along × **t**

When an end point defines the line, α = atan2(Δeasting, Δnorthing).

## Symbols and units

Coordinates, along distance, and offset use one compatible length unit: meters, international feet, or US survey feet. α is a north azimuth. Station strings use the project convention: 12+34.56 represents 1,234.56 ft, while metric stationing displays three decimal places.

## Domain

The line must have a nonzero direction. Supply either a bearing or azimuth, or both coordinates of a different end point. The point and all line coordinates must use compatible length units. The line is extended without bounds, so stations before the start and beyond the supplied end are valid mathematical results.

## Approximations

This is exact plane geometry subject to binary floating-point evaluation and display rounding. It does not model grid-to-ground scale, Earth curvature, a circular curve, a spiral, or a segmented alignment.

## Worked example

- sourcePublisher: Autodesk
- sourceTitle: Civil 3D User Guide, To Create a Point From a Survey Baseline Using a Station and Offset
- sourceEdition: Civil 3D 2024 online help
- sourceLocator: help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-5D79BE01-65E7-4A68-A807-D05B7D8CA155.htm (BL TP 3 220 15.24 and BL TP 4 270 -15.24)
- independent: yes
- inputs: baseline from N 1,000, E 1,000 at station 0+000, azimuth 100°; the page's printed points 3 (N 946.7889, E 1,214.0113) and 4 (N 968.1235, E 1,268.5445)
- outputs: point 3 at station 0+220.000, offset +15.24 m (right); point 4 at station 0+270.000, offset −15.24 m (left), each within the page's printed 1e-4 m
- tolerance: 1e-4 m against the page; 1e-9 relative against the Python evaluation in the golden vectors
- verifiedBy: golden vectors v024 and v025
- verifiedOn: 2026-10-19

## Differential tests

- `tools/vectors/gen_survey.py` independently rotates point deltas into the line frame.
- `core/vectors/survey.cogo.perpendicular-foot.jsonl` contains 20 successful projections and three refusal cases across all supported length units, plus the two Autodesk points (v024, v025).
- Cases alternate between direction-defined and two-point lines and include negative stationing.
- The integration suite requires exact parity with the projection mode of `survey.cogo.station-offset`.

## Invariants

- `core/crates/gp-survey/tests/cogo_views_invariants.rs` `station_offset_views_invariants`: at five line azimuths, the vector from the reported foot to the point is perpendicular to the line; a point built from a station and offset projects back to that station and offset; and a second point on the ray gives the same answer as a direction.
