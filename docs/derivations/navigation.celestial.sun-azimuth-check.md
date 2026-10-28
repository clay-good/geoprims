<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Compass check by the Sun (`navigation.celestial.sun-azimuth-check`)

## Method

A compass is checked by comparing a bearing it reads on a known object with the true bearing of that object. The Sun is the usual object, and there are two ways to get its true bearing.

1. **By azimuth.** The Sun's apparent place comes from the same `gp-astro` series as the almanac tool (VSOP87D with light time, annual aberration, and the FK5 correction, carried to the true equator and equinox of date). The local hour angle is the Greenwich hour angle plus the longitude, east positive, and the true azimuth follows from the navigational triangle. This works at any altitude, and is what Bowditch §2016 does by triple interpolation in Pub. No. 229.
2. **By amplitude.** When the Sun's centre is on the celestial horizon its computed altitude is zero, and the altitude equation collapses to sin A = sin d / cos L, where A is the arc of the horizon between the Sun and the prime vertical. That is Bowditch §2017, and it is the only angle in celestial navigation measured from east or west rather than from a pole. The tool reports it with its prefix and suffix, and as a true bearing.

The error is then the true azimuth less the bearing observed, named east when the compass reads low and west when it reads high. For a magnetic compass the tool also subtracts the variation supplied, leaving the deviation.

The amplitude is reported at every instant, because it is a property of the latitude and declination alone, but it is only the Sun's bearing at the moment its centre crosses the celestial horizon. The result says where to look for that moment: the Sun's centre is on the celestial horizon when its lower limb sits about two thirds of its own diameter above the visible horizon. The computed altitude is reported beside it so the reader can see how far from that moment the observation was.

## Equations

- LHA = (GHA + λ) mod 360°, λ east positive.
- Zn = atan2(−cos d sin LHA, cos L sin d − sin L cos d cos LHA), reduced to [0°, 360°).
- Z, the azimuth angle from the elevated pole: Zn or 360° − Zn in north latitude, |180° − Zn| in south, suffixed E when LHA > 180° and W when it is less.
- Hc = asin(sin L sin d + cos L cos d cos LHA), reported as the computed altitude.
- A = |asin(sin d / cos L)|, prefixed E when LHA > 180° (the Sun east of the meridian, rising) and W when it is less, suffixed N or S to agree with the declination.
- The amplitude as a true bearing: 90° − A for E…N, 90° + A for E…S, 270° + A for W…N, 270° − A for W…S.
- error = Zn − bearing observed, wrapped into ±180°, east positive.
- deviation = error − variation, wrapped into ±180°, east positive.

## Symbols and units

L is the latitude, λ the longitude, d the Sun's declination, LHA its local hour angle, Zn its true azimuth, Z the azimuth angle, Hc the computed altitude, and A the amplitude. All angles are output in degrees. Trigonometry runs in radians.

## Domain

A UTC time from 1800-01-01 to 2100-12-31 (OUT_OF_DOMAIN otherwise); DUT1 within ±0.9 s; any position; an observed bearing in [0°, 360°); a variation within ±180°. Where |sin d / cos L| exceeds 1 the Sun does not cross the celestial horizon on that date and has no amplitude, which is OUT_OF_DOMAIN. Checking a magnetic compass without a variation is INVALID_INPUT, because the error could not then be split.

## Approximations

- **The amplitude is a horizon quantity.** It holds at the instant the Sun's centre is on the celestial horizon and nowhere else. The tool computes it from the declination at the instant given, which is the right declination for that instant but not necessarily the instant of the crossing.
- **The visible horizon is not the celestial horizon.** The two-thirds-of-a-diameter rule in the result is Bowditch's, and it is a rule of thumb that depends on refraction and height of eye. Bowditch §2019 corrects an observation taken on the visible horizon from its Table 23; the tool does not, and says so.
- **Geocentric place:** the Sun's declination and hour angle are geocentric, the almanac convention. At zero altitude the parallax in azimuth is under 0.003°, well inside what a compass can be read to.
- **The deviation is only as good as the variation.** The tool takes the variation as an input rather than computing a magnetic model, so it cannot be better than the figure supplied. `geodesy.magnetic.declination` gives one.
- **No refraction in azimuth.** Refraction raises a body almost vertically, so its effect on azimuth is small, and none is applied.

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: The American Practical Navigator (Bowditch), NGA Pub. No. 9, Volume I
- sourceEdition: 2024 edition
- sourceLocator: Chapter 20, §2018, Amplitude of a Body Observed on the Celestial Horizon, the worked gyro error
- independent: yes
- inputs: latitude 51°24.6′N, the Sun's declination N 19°40.4′, setting, observed bearing 303.0° pgc. The Sun holds that declination on 2024-07-24 at 14h30m UT and is setting then at longitude 80°44.4′E, so the example runs end to end from a time and a place.
- outputs: amplitude W 32.6°N by double interpolation of Table 22, a true bearing of 302.6°, gyro error 0.4° W
- tolerance: 0.1° in the amplitude, the true bearing, and the error. Table 22 is printed to a tenth of a degree and the example reaches its value by double interpolation; the closed form gives 32.67°, which the table cannot resolve.
- verifiedBy: `core/crates/gp-navigation/tests/celestial.rs` `bowditch_amplitude_and_gyro_error`, and golden vector v001
- verifiedOn: 2026-10-27

Bowditch §2016 works the azimuth the other way, by Pub. No. 229, twice over: at DR latitude 33°24.0′N with the Sun's declination N 20°13.8′ and LHA 316°41.2′ it gives Z N 97.7° E, Zn 097.7°, and against a gyro reading 096.5° an error of 1.2° E; at latitude 33°24.0′S, with the declination contrary, Z N 135.9° E, Zn 044.1°, and against 042.5° an error of 1.6° E. The closed form reproduces both azimuths to the printed 0.1°, and both fix the sign of the error: the compass reads low in each, and the error is east.

The spec's own gyro-error scenario reads "computed azimuth 124.4°, gyro reads 124.0°, error 0.4° W". That sign is the wrong way round. Both of Bowditch's worked examples take the error as the true azimuth less the compass reading and name it east when the compass reads low, which the tool follows, so the right answer for that scenario is 0.4° E.

## Differential tests

- `tools/vectors/gen_celestial_checks.py navigation.celestial.sun-azimuth-check` builds the 31 vectors in `core/vectors/navigation.celestial.sun-azimuth-check.jsonl`: Bowditch's worked amplitude, 27 rows of USNO's celnav API, and the magnetic-without-variation, deviation, and no-amplitude scenarios.
- The USNO rows give the Sun's azimuth, declination, and hour angle at 27 instants and places from 1995 to 2045, north and south, with the Sun above and below the horizon; all within 0.05° of azimuth and 0.1′ of declination and hour angle. The azimuth is the same code path the sight-reduction tool runs, where 148 body-instants agree within 0.001°.

## Invariants

- `core/crates/gp-navigation/tests/celestial.rs` `bowditch_amplitude_and_gyro_error` checks that at the instant the Sun's computed altitude is zero, the amplitude's true bearing and the computed azimuth are the same bearing. That is the statement that the two methods in this tool are one method.
- `core/crates/gp-navigation/tests/celestial.rs` `the_compass_error_takes_its_name_from_the_compass` checks the sign of the error either side of the truth, and that it stays inside ±180° when the two bearings sit on opposite sides of north.
- `core/crates/gp-navigation/tests/celestial.rs` `a_magnetic_compass_needs_its_variation` checks that a magnetic compass is refused without a variation, that the total error does not depend on which compass is named, and that the deviation is the error less the variation.
- `core/crates/gp-navigation/tests/celestial.rs` `the_amplitude_has_a_domain` checks that the tool answers without a bearing, and that a latitude where the Sun does not cross the celestial horizon is refused rather than answered.
- `core/crates/gp-navigation/tests/celestial.rs` `bowditch_azimuth_by_tables` checks the azimuth formula at Bowditch §2016's own arguments, with the declination of the same name as the latitude and then contrary to it.
