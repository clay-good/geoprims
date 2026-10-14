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

- sourcePublisher: Pearson
- sourceTitle: Elementary Surveying: An Introduction to Geomatics
- sourceEdition: 16th edition (2021)
- sourceLocator: Chapter 11, coordinate geometry and station-offset calculation
- independent: yes
- inputs: line start N 5,000.00 ft, E 5,000.00 ft at 10+00, direction 60°; target station 13+50, offset −12.5 ft
- outputs: point N 5,185.82531755 ft, E 5,296.85889132 ft, left of the line
- tolerance: relative 1e-12 in the golden vector
- verifiedBy: golden vector v001 and the `named_direction_and_station_modes_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_survey.py` independently constructs each point from tangent and normal vectors.
- `core/vectors/survey.cogo.point-from-station-offset.jsonl` contains 20 successful setout calculations and three refusal cases across all supported length units.
- Cases alternate between direction-defined and two-point lines and cover positive, negative, and zero offsets.
- The integration suite requires exact parity with the setout mode of `survey.cogo.station-offset`.

## Invariants

- `core/crates/gp-survey/tests/cogo_views_invariants.rs` `station_offset_views_invariants`: at five line azimuths and three stations, the point less its foot is the entered signed offset along the right-hand normal; the point projects back to the entered station and offset; and a second point on the ray gives the same point as a direction.
