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

- sourcePublisher: Pearson
- sourceTitle: Elementary Surveying: An Introduction to Geomatics
- sourceEdition: 16th edition (2021)
- sourceLocator: Chapter 11, coordinate geometry and perpendicular distance
- independent: yes
- inputs: line from N 0.00 ft, E 0.00 ft at station 0+00, direction 0°; point N 50.00 ft, E −20.00 ft
- outputs: station 0+50.00, offset −20.00 ft (left), foot N 50.00 ft, E 0.00 ft
- tolerance: 1e-9 ft in the golden vector
- verifiedBy: golden vector v001 and the `named_direction_and_station_modes_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_survey.py` independently rotates point deltas into the line frame.
- `core/vectors/survey.cogo.perpendicular-foot.jsonl` contains 20 successful projections and three refusal cases across all supported length units.
- Cases alternate between direction-defined and two-point lines and include negative stationing.
- The integration suite requires exact parity with the projection mode of `survey.cogo.station-offset`.

## Invariants

- The vector from the reported foot to the point is perpendicular to the line.
- Projecting the reported point back to the line returns the reported foot and offset.
- Replacing a direction with a second point on the same ray does not change the result.
- The named operation and the general station-offset tool use the same calculation path.
