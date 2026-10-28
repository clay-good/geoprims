<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Moon position (`time.moon.position`)

## Method

Where the moon is in the sky, as seen from a place on the ground rather than from the centre of the Earth. The difference matters for the moon and almost nothing else: it is close enough that an observer on the surface sees it up to about 57′ away from where a geocentric almanac puts it, which is nearly two of its own diameters.

1. The moon's apparent geocentric right ascension, declination, and distance come from the same `gp-astro` series as the celestial almanac: ELP 2000-82B carried to the true equator and equinox of date by IAU 2006 precession and IAU 2000A nutation.
2. The observer's own position is built as a vector in that same frame, from the WGS 84 ellipsoid at the given latitude, longitude, and height, with the local apparent sidereal time as its right ascension.
3. That vector is subtracted from the moon's. What is left is the direction and distance from the observer, which is the topocentric place.
4. The topocentric direction is turned into azimuth and altitude with the local hour angle and the geodetic latitude, and refraction is added to give the altitude a sextant or a camera would see.

Subtracting vectors rather than applying a parallax correction in altitude is what makes the azimuth right as well. Parallax moves the moon along the vertical circle through the observer's **geocentric** zenith, while azimuth is measured from the **geodetic** vertical, and the two differ by up to 11.5′; a correction applied in altitude alone would leave that difference in the azimuth.

## Equations

- HP = asin(a / d), SD = asin(0.2725076 sin HP), with a the Earth's equatorial radius and d the geocentric distance.
- The observer, in the true equator of date: N = a / sqrt(1 − e² sin²φ), and the vector ((N + h) cos φ cos θ, (N + h) cos φ sin θ, (N(1 − e²) + h) sin φ) with θ = GAST + λ.
- The moon: d (cos δ cos α, cos δ sin α, sin δ).
- The topocentric vector is their difference; its length is the distance reported, and its direction gives α′ and δ′.
- H = θ − α′, then sin h = sin φ sin δ′ + cos φ cos δ′ cos H and Zn = atan2(−cos δ′ sin H, cos φ sin δ′ − sin φ cos δ′ cos H).
- The apparent semidiameter is scaled for the observer's own distance: asin(sin SD × d / r), with r the topocentric distance.

## Symbols and units

φ is the geodetic latitude, λ the longitude, h the height above the ellipsoid, α and δ the geocentric right ascension and declination, α′ and δ′ the topocentric ones, θ the local apparent sidereal time, HP the horizontal parallax, and SD the semidiameter. Angles are output in degrees, distance in kilometres, parallax and semidiameter in minutes of arc. Trigonometry runs in radians.

## Domain

A UTC time from 1800-01-01 to 2100-12-31 (OUT_OF_DOMAIN otherwise); any place; any height above the ellipsoid. Delta T is taken from the given value, or estimated for the date.

## Approximations

- **Refraction** is the NOAA formula the sun tools use, for 1013.25 hPa and 10 °C. Near the horizon the real value moves by minutes of arc with the weather, so the refracted altitude of a low moon is approximate; the geometric altitude beside it is not.
- **The place is the moon's centre**, not a limb. The semidiameter is reported so a limb can be worked out.
- **Light time** is already in the apparent place from the series; no further correction is applied here.
- **A level horizon.** The altitude is measured from the astronomical horizon, so terrain and buildings hide a moon well above zero.

## Worked example

- sourcePublisher: US Naval Observatory, Astronomical Applications Department
- sourceTitle: USNO Astronomical Applications API, celnav
- sourceEdition: API 4.0.1
- sourceLocator: Celestial-navigation data for the moon at 129 places and instants, giving its geocentric computed altitude and its own parallax in altitude
- independent: yes
- inputs: the moon at 1997-08-22T19:35:24Z from 15.2239 N, 134.8606 E
- outputs: USNO's geocentric altitude less its own parallax in altitude is the topocentric altitude, which the tool reproduces to 0.034′; its declination to 0.011′
- tolerance: 0.05′ in topocentric altitude and 0.02′ in declination
- verifiedBy: `core/crates/gp-time/tests/moon.rs` `position_matches_usno`, and golden vectors v001 to v026
- verifiedOn: 2026-10-28

## Differential tests

- `core/crates/gp-time/tests/moon.rs` `position_matches_usno` runs the tool against USNO's celnav data at 129 places and instants from 1995 to 2045. The topocentric altitude is within 0.034′ and the declination within 0.011′.
  - USNO publishes the azimuth of the moon's **geocentric** place. It sits up to 0.36′ from the topocentric azimuth, growing as 1/cos(altitude), because the parallax direction is tilted by the difference between the geodetic and geocentric verticals. The test measures that below 85° of altitude and leaves it alone above, where a hundredth of a degree of altitude swings the azimuth by degrees.
- `tools/vectors/gen_moon.py time.moon.position` builds the 27 vectors in `core/vectors/time.moon.position.jsonl`: every fifth celnav row, and the out-of-domain scenario.

## Invariants

- `core/crates/gp-time/tests/moon.rs` `position_matches_usno` checks the declination against USNO's at every row, which is the statement that the geocentric place behind this tool is the one the celestial almanac publishes.
- The topocentric distance is always shorter than the geocentric one when the moon is up and longer when it is down, which the azimuth and altitude checks exercise across 129 rows above and below the horizon.
