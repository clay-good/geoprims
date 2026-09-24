<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Inverse between two coordinates (`survey.cogo.inverse`)

## Method

Plane coordinate geometry. The differences in northing and easting between the two points are the latitude and departure of the line. The azimuth is the direction of that vector from grid north, and the distance is its length. The quadrant bearing is the same direction written from north or south toward east or west, rounded to the second. US survey feet and international feet may not be mixed; other units convert to the first point's unit.

## Equations

- ΔN = N₂ − N₁, ΔE = E₂ − E₁
- Azimuth = atan2(ΔE, ΔN), wrapped to [0°, 360°)
- Distance = √(ΔN² + ΔE²), computed with hypot so large coordinates do not overflow or lose digits
- Bearing: N az E for 0° to 90°, S (180° − az) E to 180°, S (az − 180°) W to 270°, N (360° − az) W to 360°

## Symbols and units

N northing and E easting, in one length unit (ft, ftUS, m, or another length). The distance comes back in that unit. The azimuth is in degrees clockwise from grid north.

## Domain

Any two plane coordinates. When the points coincide the distance is zero and there is no direction, which the result flags with AZIMUTH_UNDEFINED.

## Approximations

None on the plane. The bearing and distance are grid values: the ground distance is the grid distance divided by the combined factor, and grid north differs from true north by the convergence angle.

## Worked example

- sourcePublisher: University of Memphis, Department of Civil Engineering
- sourceTitle: CIVL 1112 Surveying - Traverse Calculations (course notes)
- sourceEdition: retrieved 2026-09-24
- sourceLocator: Page 4, the latitudes and departures table: course AB, S 6°15′ W, 189.53 ft, latitude −188.403, departure −20.634, and courses BC through EA
- independent: yes
- inputs: from (0, 0) to northing −188.403, easting −20.634 ft
- outputs: distance 189.53 ft and azimuth 186.25° (S 6°15′ W), within the table's 0.001 ft rounding; the other four courses likewise
- tolerance: 0.001 ft on distance, 0.0005° on azimuth (the table rounds latitudes and departures to 0.001 ft)
- verifiedBy: golden vectors v007 to v011, run by the core on every build
- verifiedOn: 2026-09-24

## Differential tests

- `tools/vectors/gen_survey.py`: an independent Python implementation (atan2 and hypot) at ten random state-plane-sized pairs in every quadrant, with bearings to the second (within 1e-9 relative)
- `core/vectors/survey.cogo.inverse.jsonl`: those vectors, the published table, due east and due south, meters and US survey feet kept in their own unit, and the mixed-feet refusal, run through the core on every build

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `inverse_forward_invariants`: forward undoes inverse; swapping the points turns the azimuth by 180° and keeps the distance; shifting both points changes nothing; turning the pair about the origin adds the same turn to the azimuth
