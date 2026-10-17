<!-- Derivation note (trust/correctness-program layer A). -->
# Survey bearing and azimuth converter (`survey.cogo.direction-parse`)

## Method

A survey direction is read as a north azimuth, measured clockwise from north. A quadrant bearing starts from north or south and turns by an acute angle toward east or west. A gon value uses 400 gons for one circle. The tool preserves the direction reference the user supplies; it changes notation, not the north reference.

## Equations

- N θ E: azimuth = θ
- S θ E: azimuth = 180° − θ
- S θ W: azimuth = 180° + θ
- N θ W: azimuth = 360° − θ
- degrees = 0.9 × gons

Every azimuth is wrapped to [0°, 360°). The inverse quadrant conversion selects the quadrant containing the azimuth and reports its acute angle.

## Symbols and units

θ is the quadrant angle, from 0° through 90°. Azimuths are degrees clockwise from north. Gons are angular units with 400 gon = 360°. The direction reference is `grid`, `geodetic`, or `assumed`.

## Domain

Quadrant bearings must begin with N or S, end with E or W, and contain an angle from 0° through 90°. North azimuths must be from 0° through 360°. Gon values must be from 0 through 400. Decimal degrees, degrees-minutes-seconds, dash-separated DMS, `gon`, `grad`, and `g` suffixes are accepted.

## Approximations

The conversion itself is exact. The DMS strings are displayed to 0.01 arcsecond and decimal outputs to eight places. No convergence, declination, or basis-of-bearing rotation is applied.

## Worked example

- sourcePublisher: Pearson
- sourceTitle: Elementary Surveying: An Introduction to Geomatics
- sourceEdition: 16th edition (2021)
- sourceLocator: Direction notation and azimuth conversion in the angle-measurement and traverse chapters
- independent: yes
- inputs: S 44°30′00″ W, grid reference
- outputs: grid azimuth 224°30′00″, quadrant bearing S 44°30′00″ W, 249.44444444 gon
- tolerance: exact before display rounding
- verifiedBy: golden vector v008 and the `inverse_and_bearings` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_survey.py` evaluates each quadrant equation and the degree-to-gon ratio independently for cardinal, diagonal, DMS, decimal, and gon inputs.
- `core/vectors/survey.cogo.direction-parse.jsonl` holds 21 successful conversions and five refusal cases.

## Invariants

- `core/crates/gp-survey/tests/cogo_views_invariants.rs` `direction_parse_invariants`: cardinal directions land at 0°, 90°, 180°, and 270° from either neighboring quadrant; 0 gon and 400 gon both wrap to north while 100, 200, and 300 gon are east, south, and west; a valid quadrant bearing converted to an azimuth and read back from the reported bearing preserves the direction to the displayed 0.01 arcsecond; and the reference label passes through unchanged because this operation does not rotate the direction.
