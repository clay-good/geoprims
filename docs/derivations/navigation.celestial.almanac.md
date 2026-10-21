<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Celestial almanac (`navigation.celestial.almanac`)

## Method

A navigator reduces a sextant sight with the Greenwich hour angle (GHA) and declination of the body at the second of the sight, which the Nautical Almanac tabulates hourly. This tool computes them instead, for one UTC instant, from open series in the `gp-astro` library.

- **Sun and planets:** VSOP87D (Bretagnon and Francou 1988), truncated by amplitude at 3e-7 (1,989 of 18,832 terms). The planet is taken at t − τ less the Earth at t (light time), with annual aberration (Meeus eq. 23.2), the VSOP-to-FK5 correction (Meeus eq. 32.3), nutation in longitude, and the true obliquity.
- **Moon:** ELP 2000-82B (Chapront-Touze and Chapront), truncated at 1e-7 rad (769 of 37,872 terms) by the authors' own rule in `elp82b.f`. It is evaluated at t − τ, rotated to the J2000 ecliptic as their subroutine does, then carried to the true equator of date. For a body this close, light time from its geocentric motion is the whole aberration.
- **Stars:** the 57 navigational stars and Polaris from Hipparcos (van Leeuwen 2007). Each is moved by proper motion from J1991.25, carried to the true equator of date, then corrected for parallax, the Sun's light deflection, and annual aberration from the Earth's VSOP87 position and velocity.
- **Precession and nutation:** IAU 2006 precession by the Fukushima-Williams angles (IERS eq. 5.39 and 5.40), and the full IAU 2000A nutation from IERS Tables 5.3a and 5.3b.
- **Hour angles:** GHA = GAST − RA, with GAST by IAU 2006/2000A; GHA Aries = GAST; SHA = 360° − RA.
- **Time scales:** UT1 = UTC + DUT1, with UT1_APPROXIMATED if DUT1 is not given. TT = UTC + (TAI − UTC) + 32.184 s from 1972, from the leap-second table echoed in `meta.assets`. Before 1972, TT = UT1 + the Espenak and Meeus ΔT estimate.

## Equations

- GHA = (GAST − α) mod 360°; SHA = (360° − α) mod 360°; GHA star = GHA Aries + SHA.
- Moon: HP = asin(6,378.137 km / Δ) and SD = asin(0.2725076 sin HP), with Δ the geocentric distance.
- Sun: SD = 959.63″ / R, with R in au.
- Planets: HP = 8.794143″ / Δ, with Δ in au.
- Equation of time E = (GHA_Sun − 180° − 15° × UT1 hours), wrapped to ±180° and converted at 4 minutes per degree. It is positive when the Sun is ahead of the mean sun.
- Planet aberration (Meeus 23.2): Δλ = (−κ cos(⊙ − λ) + eκ cos(π − λ)) / cos β and Δβ = −κ sin β (sin(⊙ − λ) − e sin(π − λ)), with κ = 20.49552″.
- Light deflection: u′ = u + (g / E)(e − (u·e) u) / (1 + u·e), with g = 2GM☉/c² per au = 1.974e-8 rad, e the unit vector from the Sun to the Earth, and E the Earth's distance from the Sun.
- Annual aberration of a star: u′ = u + v/c − (u·v/c) u, with v the Earth's velocity by central difference of VSOP87 over ±0.05 day.

## Symbols and units

Angles are in radians internally. GHA, SHA, and declination are output in degrees (north positive), shown in degrees and minutes in the answer. HP and SD are in arcminutes, the equation of time in minutes, and ΔT in seconds.

## Domain

A UTC time from 1800-01-01 to 2100-12-31 (OUT_OF_DOMAIN otherwise); DUT1 within ±0.9 s (INVALID_INPUT otherwise). After the leap-second table's end (2027-06-30), LEAP_SECOND_TABLE_EXPIRED warns that a later leap second would shift GHA by up to 0.25′.

## Approximations

- **Truncation:** cutting the series costs at most 1.6″ against JPL Horizons over 1950 to 2050, and 0.7″ for the Moon. The almanac prints to 0.1′ (6″).
- **What is left out:** stars' radial velocity, second-order aberration, and the Sun's deflection of planetary light. Each is below 0.02″ for navigation stars and bodies.
- **Geocentric places:** the reader's parallax in altitude is the altitude-correction step, not part of the almanac data.
- **Leap-second table:** it ends at 2027-06-30. Later dates keep the last TAI − UTC.
- **USNO's departures:** USNO's own values depart from JPL in two places, as the change's design records:
  - Venus near inferior conjunction, by up to 0.4′.
  - The Moon after about 2030, by about 0.005′ a year, likely from its predicted ΔT.

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: The American Practical Navigator (Bowditch), NGA Pub. No. 9, Volume I
- sourceEdition: 2024 edition
- sourceLocator: Chapter 20, §2010, Tables 2010a and 2010b (four-star fix of 2024-06-02); §2011, Table 2011 (the Sun on 2024-05-31)
- independent: yes
- inputs: 2024-06-02T08:24:03Z with DUT1 0 s
- outputs: GHA Aries 17°18.7′ (11°17.0′ at 08h plus 6°01.7′); Spica SHA 158°22.6′, Dec S 11°17.4′; Regulus 207°35.1′, N 11°50.9′; Procyon 244°51.6′, N 5°09.8′ (Table 2010a; Table 2010b repeats its GHA as the SHA); Canopus 263°53.1′, S 52°42.6′; the Sun on 2024-05-31 at 15h24m13s, GHA 51°36.5′, Dec N 22°02.6′. Table 2011's GHA for 15h15m03s, 49°22.0′, is a misprint; USNO gives 49°18.9′.
- tolerance: 0.1′, the almanac's printed resolution
- verifiedBy: `core/crates/gp-navigation/tests/celestial.rs` `bowditch_four_star_fix` and `bowditch_high_altitude_sun`, and golden vectors v001 to v003
- verifiedOn: 2026-10-21

## Differential tests

- `core/crates/gp-navigation/tests/celestial.rs` `matches_usno_through_the_tool` compares the tool with USNO's celnav API at 40 instants from 1995 to 2045 (times as UT1).
  - Sun, planets, Aries, and 57 stars: worst 0.018′, within 0.1′.
  - Venus: within 0.5′.
  - The Moon: within 0.25′.
- `core/crates/gp-astro/tests/apparent.rs` compares the series with JPL Horizons (DE441) apparent RA and Dec, 1950 to 2050.
  - Sun and planets at 204 body-instants: within 1.6″.
  - The Moon at 60 instants: within 0.7″.
  - The IAU 2006/2000A nutation and precession angles match ERFA within 1e-11 rad.
- `core/crates/gp-astro/tests/stars.rs` compares the star reduction with ERFA's `pmsafe` and `atci13` at 1,160 star-instants: within 0.015″.
- `tools/vectors/gen_celestial.py` builds the 14 vectors in `core/vectors/navigation.celestial.almanac.jsonl`: the Bowditch values, ten USNO instants, and the out-of-domain scenario.

## Invariants

- `core/crates/gp-navigation/tests/celestial.rs` `hp_sd_and_the_equation_of_time` checks:
  - The equation of time is +16.4 min on 2024-11-03 and −14.2 min on 2024-02-12.
  - The Moon's HP lies within its 53.9′ to 61.5′ range, and its SD is 0.2725 HP.
  - The Sun's SD lies between 15.7′ and 16.3′.
  - The tool returns 6 bodies and 58 stars.
- `core/crates/gp-navigation/tests/celestial.rs` `out_of_range_and_warnings` checks:
  - 1799 and 2101 are OUT_OF_DOMAIN, naming 1800 to 2100.
  - UT1_APPROXIMATED and LEAP_SECOND_TABLE_EXPIRED appear in 2030 without DUT1.
  - ΔT in 1850 is the dated estimate.
