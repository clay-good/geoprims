<!-- Derivation note (trust/correctness-program layer A). -->
# Azimuth and bearing converter (`survey.cogo.azimuth-bearing`)

## Method

The operation reads one survey direction and expresses that same direction as a north azimuth, a degrees-minutes-seconds azimuth, a quadrant bearing, and gons. A north azimuth is measured clockwise from north. A quadrant bearing starts at north or south and turns through an acute angle toward east or west. The selected grid, geodetic, or assumed reference is carried through unchanged.

## Equations

- N θ E: azimuth = θ
- S θ E: azimuth = 180° − θ
- S θ W: azimuth = 180° + θ
- N θ W: azimuth = 360° − θ
- gons = azimuth / 0.9

Azimuths are reduced to the half-open interval [0°, 360°). The inverse conversion chooses the quadrant containing the azimuth and uses its acute angle θ.

## Symbols and units

θ is the quadrant angle from 0° through 90°. Azimuths use degrees clockwise from north. One full circle is 360° or 400 gon. A direction may be labeled `grid`, `geodetic`, or `assumed`; the label does not alter the numeric conversion.

## Domain

The parser accepts decimal degrees, degrees-minutes-seconds, dash-separated DMS, and values ending in `gon`, `grad`, or `g`. Quadrant bearings must start with N or S, end with E or W, and contain an angle no greater than 90°. North azimuth inputs may include 360°, which wraps to 0°.

## Approximations

The angular conversion is exact. Displayed DMS values are rounded to 0.01 arcsecond and decimal values to eight places. The operation does not apply grid convergence, magnetic declination, or a basis-of-bearing rotation.

## Worked example

- sourcePublisher: Pearson
- sourceTitle: Elementary Surveying: An Introduction to Geomatics
- sourceEdition: 16th edition (2021)
- sourceLocator: Direction notation and azimuth conversion in the angle-measurement and traverse chapters
- independent: yes
- inputs: N 10°15′30″ W, assumed reference
- outputs: assumed azimuth 349°44′30″, quadrant bearing N 10°15′30″ W, 388.60185185 gon
- tolerance: exact before display rounding
- verifiedBy: golden vector v001 and the `named_direction_and_station_modes_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_survey.py` evaluates all four quadrant equations and the degree-to-gon ratio independently.
- `core/vectors/survey.cogo.azimuth-bearing.jsonl` covers 21 valid directions and five refusal cases.
- The integration suite requires byte-for-byte result, display, and warning parity with `survey.cogo.direction-parse` for the same input.

## Invariants

- Cardinal directions agree at the boundaries of adjacent quadrants.
- 0 gon and 400 gon both map to north.
- Changing the reference label does not change the numeric direction.
- The named operation and the general direction parser use the same calculation path.
