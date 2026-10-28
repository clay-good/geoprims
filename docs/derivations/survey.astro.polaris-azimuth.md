<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Astronomic azimuth from a Polaris observation (`survey.astro.polaris-azimuth`)

## Method

An astronomic azimuth is got by pointing a theodolite at a star whose direction can be computed, pointing it at the reference mark, and subtracting. The arithmetic is three steps, and Coast and Geodetic Survey Special Publication 237 lays them out: "the computation of the azimuth of a star at the instant of observation, the computation of the horizontal angle between the star and the mark, and the combination of these two results by addition or subtraction."

1. Polaris's apparent place comes from the same `gp-astro` series as the celestial almanac: Hipparcos (van Leeuwen 2007) with proper motion, parallax, light deflection, and aberration, carried to the true equator and equinox of date by IAU 2006 precession and IAU 2000A nutation.
2. Its local hour angle is the Greenwich hour angle plus the longitude, east positive, with the Greenwich apparent sidereal time of the instant.
3. Its azimuth from north, clockwise, is the solution of the astronomical triangle at the zenith, which SP 237 gives in the form used here.
4. The line's azimuth is the star's azimuth less the horizontal angle from the mark to the star.
5. With more than one pointing, the azimuths are meaned round the circle, the scatter is reported, and a pointing that disagrees with the rest is flagged.

SP 237 prefers Polaris over any other body because its first term, cos φ tan δ, "changes very slowly on account of the very small change in the declinations during the period of observation."

## Equations

- LHA = (GHA + λ) mod 360°, λ east positive, GHA from the Greenwich apparent sidereal time less the apparent right ascension.
- A = atan2(−sin t, cos φ tan δ − sin φ cos t), from north clockwise, with t the local hour angle. This is SP 237's tan A = −sin t / (cos φ tan δ − sin φ cos t) written so the quadrant comes out right without a rule.
- The line: A_line = (A_body − angle) mod 360°, angle measured clockwise from the mark to the body.
- The mean of a set is the circular mean, atan2(Σ sin A, Σ cos A), because a plain mean of azimuths either side of north gives the reciprocal.
- The residual of a pointing is its distance from the mean the short way round the circle, in seconds of arc.
- Geodetic azimuth = astronomic azimuth + the Laplace correction supplied.

## Symbols and units

φ is the geodetic latitude, λ the longitude, δ the body's apparent declination, t its local hour angle, and A an azimuth from true north clockwise. Azimuths are output in degrees to five places and also written in degrees, minutes, and seconds; residuals, scatter, and the Laplace correction in seconds of arc. Trigonometry runs in radians.

## Domain

Any geodetic position, though Polaris is only observable from the northern hemisphere; a UTC instant per pointing from 1800-01-01 to 2100-12-31 (OUT_OF_DOMAIN otherwise); a horizontal angle from 0° up to 360°; 1 to 200 pointings (LIMIT_EXCEEDED beyond). Without DUT1 the result carries `UT1_APPROXIMATED`. Every result carries `ASTRONOMIC_AZIMUTH`.

## Approximations

- **The azimuth is geocentric in the sense that matters.** A star has no parallax to speak of, so the direction computed is the direction seen.
- **No refraction.** Refraction raises a body almost vertically, so its effect on azimuth is small, and none is applied. It is not nil for a body at low altitude near the horizon, which is one reason SP 237 wants Polaris and not a low star.
- **The Laplace correction is an input, not a computation.** It depends on the deflection of the vertical at the station, which needs a geoid model and a longitude determination; the tool reports the astronomic azimuth and, given the correction, the geodetic one.
- **The blunder test is a rule of thumb.** Each pointing is judged against the scatter of the others, which catches one gross error in a set but is not a substitute for SP 237's specifications: at least 24 acceptable observations over two nights, none 5 seconds of arc or more from the mean.
- **One pointing is not a determination.** With a single pointing the scatter is zero, which says nothing about the quality of the azimuth.

## Worked example

- sourcePublisher: US Naval Observatory, Astronomical Applications Department
- sourceTitle: USNO Astronomical Applications API, celnav
- sourceEdition: API 4.0.1
- sourceLocator: Azimuth of Polaris for 2016-02-23 04:21:15 UT at 29.516667, −74.5
- independent: yes
- inputs: latitude 29°31.0′N, longitude 074°30.0′W, one pointing at 2016-02-23T04:21:15Z with a horizontal angle of 45°00′00″
- outputs: USNO puts Polaris at 359.250169°, so the line is at 314.250°. The tool returns Polaris at 359.25017° and the line at 314.25017°.
- tolerance: 0.001° (3.6 seconds of arc), which is finer than the spec's 0.1°
- verifiedBy: `core/crates/gp-survey/tests/astro.rs` `the_spec_polaris_observation` and `azimuth_matches_usno`, and golden vector v001
- verifiedOn: 2026-10-28

This is the observation the change's spec gives as "Bowditch §1504". That section number is the 2019 edition's, and no such worked example could be found in the 2024 edition's text, so the check rests on USNO's own figure for the same instant and place rather than on the book. The spec's expected values, Polaris at 359.2° and the line at 314.2°, are both confirmed.

## Differential tests

- `core/crates/gp-survey/tests/astro.rs` `azimuth_matches_usno` runs both tools against USNO's celestial-navigation data at 54 places and instants from 1995 to 2045 (34 on Polaris, 20 on the sun), with a single pointing at a zero horizontal angle so the line's azimuth is the body's. The worst disagreement is 0.000346°, which is 1.2 seconds of arc.
- `tools/vectors/gen_survey_astro.py survey.astro.polaris-azimuth` builds the 37 vectors in `core/vectors/survey.astro.polaris-azimuth.jsonl`: the spec's worked observation, every USNO Polaris row above a degree of altitude, and the out-of-domain and bad-angle scenarios.

## Invariants

- `core/crates/gp-survey/tests/astro.rs` `a_blunder_in_a_set_is_flagged` checks that one pointing 2 minutes of arc out of six is flagged with `SUSPECT_VALUE`, that it is the fifth and only the fifth, and that the mean without it moves by the expected 20 seconds of arc.
- `core/crates/gp-survey/tests/astro.rs` `a_laplace_correction_gives_the_geodetic_azimuth` checks that the geodetic azimuth is the astronomic one plus the correction to a millionth of a degree, that it is absent when no correction is given, and that the warning names the value.
- `core/crates/gp-survey/tests/astro.rs` `the_inputs_are_checked_row_by_row` checks that a bad time or angle is refused at its own row's pointer, that a time outside 1800 to 2100 is OUT_OF_DOMAIN, and that a set without DUT1 says UT1 was approximated.
