<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Astronomic azimuth from a solar observation (`survey.astro.solar-azimuth`)

## Method

The same three steps as a Polaris observation, and the same engine: compute the body's azimuth at the instant of the pointing, subtract the horizontal angle from the reference mark, and mean the set. One thing is different, and it is the reason a solar observation needs its own tool: the sun is half a degree wide, so a pointing is made on a limb, not on the centre.

1. The sun's apparent place comes from the same `gp-astro` series as the celestial almanac: VSOP87D with light time, annual aberration, and the FK5 correction, carried to the true equator and equinox of date by IAU 2006 precession and IAU 2000A nutation.
2. Its local hour angle is the Greenwich hour angle plus the longitude, east positive.
3. Its azimuth from north, clockwise, follows the hour-angle form of Coast and Geodetic Survey Special Publication 237.
4. A pointing on a limb is carried to the centre. A displacement of one semidiameter across the sun's disc, measured along the horizon rather than along the vertical circle, subtends SD / cos h of azimuth at altitude h. The leading limb sits at the greater azimuth, so the angle to the centre is less by that amount; the trailing limb, greater by it.
5. The line's azimuth is the sun's azimuth less the corrected horizontal angle, and the set is meaned round the circle with its scatter.

## Equations

- LHA = (GHA + λ) mod 360°, λ east positive.
- A = atan2(−sin t, cos φ tan δ − sin φ cos t), from north clockwise, with t the local hour angle (SP 237).
- h = asin(sin φ sin δ + cos φ cos δ cos t), the altitude the limb correction needs.
- SD = 959.63″ / d with d the sun's distance in astronomical units, its semidiameter at the instant.
- The limb correction is SD / cos h, subtracted from the horizontal angle for the leading limb and added for the trailing one.
- The line: A_line = (A_body − angle_to_centre) mod 360°.
- The mean of a set is the circular mean; residuals are distances from it the short way round, in seconds of arc.
- Geodetic azimuth = astronomic azimuth + the Laplace correction supplied.

## Symbols and units

φ is the geodetic latitude, λ the longitude, δ the sun's apparent declination, t its local hour angle, h its altitude, SD its semidiameter, and A an azimuth from true north clockwise. Azimuths are output in degrees to five places and in degrees, minutes, and seconds; the limb correction, residuals, scatter, and the Laplace correction in seconds of arc. Trigonometry runs in radians.

## Domain

Any geodetic position; a UTC instant per pointing from 1800-01-01 to 2100-12-31 (OUT_OF_DOMAIN otherwise); a horizontal angle from 0° up to 360°; 1 to 200 pointings (LIMIT_EXCEEDED beyond); a limb of `center`, `leading`, or `trailing`. Without DUT1 the result carries `UT1_APPROXIMATED`. Every result carries `ASTRONOMIC_AZIMUTH`.

## Approximations

- **The limb correction uses the geocentric semidiameter and the geometric altitude.** The augmentation of the sun's semidiameter is nil to the precision that matters, and the altitude used is unrefracted. The correction's own size is about 22 minutes of arc at a 45° altitude, so a tenth of a degree of error in the altitude moves it by well under a second of arc.
- **No refraction in azimuth.** Refraction raises a body almost vertically, so none is applied.
- **The sun's parallax in azimuth is ignored.** At 8.8″ of horizontal parallax its effect on azimuth is under a hundredth of a second of arc.
- **The Laplace correction is an input, not a computation.** It depends on the deflection of the vertical at the station.
- **A solar azimuth is not a first-order azimuth.** The sun is wide, its azimuth rate falls near local noon, and its altitude makes refraction and pointing error matter. SP 237's first-order Laplace specifications want a circumpolar star, at least 24 acceptable observations over two nights, and none 5 seconds of arc or more from the mean. A solar set will not meet them, and the tool's limitations say so.

## Worked example

- sourcePublisher: US Naval Observatory, Astronomical Applications Department
- sourceTitle: USNO Astronomical Applications API, celnav
- sourceEdition: API 4.0.1
- sourceLocator: The sun's azimuth, semidiameter, and altitude for 2026-06-21 15:40:00 UT at 39.739194, −104.990306
- independent: yes
- inputs: latitude 39°44′21.1″N, longitude 104°59′25.1″W, one pointing at 2026-06-21T15:40:00Z on the leading limb with a horizontal angle of 112°18′40″
- outputs: USNO gives the sun's azimuth as 95.458546°, its semidiameter as 0.262323°, and its geocentric altitude as 44.707665°, so the limb correction must be 0.262323° × 3600 / cos(44.707665°) = 1,328.3″. The tool returns an azimuth of 95.45855° and a correction of 1,328.71″.
- tolerance: 0.001° on the azimuth and 2″ on the limb correction, the latter covering the small difference between this core's semidiameter and altitude and USNO's own
- verifiedBy: `core/crates/gp-survey/tests/astro.rs` `the_limb_correction_is_the_semidiameter_over_cos_altitude` and `azimuth_matches_usno`, and golden vectors v021 and v022
- verifiedOn: 2026-10-28

The spec's own limb scenario is arithmetic rather than an observation: a semidiameter of 16.1′ at 30° of altitude gives 16.1 / cos 30° = 18.59′. The test checks that identity directly as well, since the tool cannot be made to use a semidiameter and altitude of its reader's choosing.

## Differential tests

- `core/crates/gp-survey/tests/astro.rs` `azimuth_matches_usno` runs both tools against USNO's celestial-navigation data at 54 places and instants from 1995 to 2045 (20 on the sun, 34 on Polaris), with a single pointing at a zero horizontal angle so the line's azimuth is the body's. The worst disagreement is 0.000346°, which is 1.2 seconds of arc, and the worst row is a solar one.
- `tools/vectors/gen_survey_astro.py survey.astro.solar-azimuth` builds the 23 vectors in `core/vectors/survey.astro.solar-azimuth.jsonl`: every USNO solar row above a degree of altitude, the two limb vectors derived from USNO's own semidiameter and altitude, and the Laplace scenario.

## Invariants

- `core/crates/gp-survey/tests/astro.rs` `the_limb_correction_is_the_semidiameter_over_cos_altitude` checks that the two limbs share one magnitude, that they move the line's azimuth opposite ways by exactly that much, and that the correction over the sun's semidiameter is the secant of the altitude, which is greater than 1.
- `core/crates/gp-survey/tests/astro.rs` `a_blunder_in_a_set_is_flagged`, `a_laplace_correction_gives_the_geodetic_azimuth`, and `the_inputs_are_checked_row_by_row` exercise the shared engine, which both tools run.
