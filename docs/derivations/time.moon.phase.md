<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Moon phase and illumination (`time.moon.phase`)

## Method

Two different questions, and they have two different answers.

**How much is lit** is geometry at the moon. The lit fraction of the disc depends on the phase angle, the angle at the moon between the directions of the Earth and the sun. That is not the same as the elongation seen from here, because the Earth and the sun are at very different distances, so Meeus solves the triangle: the elongation ψ from the two apparent places, then the phase angle i from ψ and the two distances, then the fraction from i.

**Which phase it is, and when the next one falls**, is the convention the almanacs use: the phases are the instants when the apparent geocentric ecliptic **longitude** of the moon is 0°, 90°, 180°, or 270° ahead of the sun's. That is a different quantity from the elongation above, which is the full angle on the sky and includes the moon's ecliptic latitude.

Both places come from the same `gp-astro` series as the celestial almanac: ELP 2000-82B for the moon, VSOP87D for the sun, carried to the true equator and equinox of date.

Finding the four instants is a root search, and each step of it costs a full run of both series. The elongation in longitude runs through 360° every synodic month at a rate that varies by about a tenth either way, so the mean rate puts a first guess within a day, and a secant iteration closes from there in three or four steps. Bisection would reach the same answers and did at first, but it cost about eight times as many runs of the series: the tool measured 42.8 ms a call and now measures 6.6 ms, with the same agreement against USNO.

## Equations

- cos ψ = sin δ_m sin δ_s + cos δ_m cos δ_s cos(α_s − α_m), the elongation seen from the Earth.
- tan i = R sin ψ / (Δ − R cos ψ), the phase angle at the moon (Meeus 48.3), with R the Earth-sun distance and Δ the Earth-moon distance.
- k = (1 + cos i) / 2, the illuminated fraction (Meeus 48.2).
- λ = atan2(sin α cos ε + tan δ sin ε, cos α), the apparent ecliptic longitude, with ε the true obliquity of date.
- The elongation in longitude is (λ_moon − λ_sun) mod 360°: 0 new, 90 first quarter, 180 full, 270 last quarter.
- The age is the time since that elongation last passed 0.

## Symbols and units

α and δ are apparent right ascension and declination, λ apparent ecliptic longitude, ε the true obliquity, ψ the elongation on the sky, i the phase angle, k the illuminated fraction, R and Δ the distances to the sun and the moon. The fraction is a number from 0 to 1; angles are in degrees, the age in days. Trigonometry runs in radians.

## Domain

A UTC time from 1800-01-01 to 2100-12-31 (OUT_OF_DOMAIN otherwise). Delta T is taken from the given value, or estimated for the date. The four instants returned are the next ones after the instant given, so they span a little over a synodic month.

## Approximations

- **The secant search** stops when the elongation is within a hundred-thousandth of a day of the target in time, under a second, or after eight steps. The measured agreement with USNO over 208 instants is the check that it converges.
- **The disc as a circle.** The illuminated fraction is of the disc as a geometric circle. It is not how bright the moon looks: the real brightness is far from proportional to it, because of the opposition surge and the texture of the surface.
- **Geocentric phases.** The instants are geocentric, the almanac convention, so they are the same for everyone on Earth. The topocentric fraction differs by a few parts in ten thousand.
- **Phase names** are the conventional eight, with the four quarters given a degree of elongation either side so an instant within about two hours of a quarter is named as that quarter.

## Worked example

- sourcePublisher: US Naval Observatory, Astronomical Applications Department
- sourceTitle: USNO Astronomical Applications API, moon/phases/date
- sourceEdition: API 4.0.1
- sourceLocator: The instants of the moon's phases, 208 of them over 1995 to 2045
- independent: yes
- inputs: 2026-06-01T00:00:00Z
- outputs: USNO puts the full moon of that month at 2026-06-29 23:56Z; the tool returns 23:57Z. At each of the 52 full moons in the sample the tool reports over 99% of the disc lit.
- tolerance: 1 minute on the instants, 0.01 on the illuminated fraction at a phase
- verifiedBy: `core/crates/gp-time/tests/moon.rs` `phases_match_usno` and `a_full_moon_is_full`, and golden vectors v001 to v030
- verifiedOn: 2026-10-28

## Differential tests

- `core/crates/gp-time/tests/moon.rs` `phases_match_usno` asks the tool, on the morning of each of USNO's 208 published phase instants from 1995 to 2045, for the next instant of that phase. Every one agrees within a minute, which is the precision USNO prints.
- `tools/vectors/gen_moon.py time.moon.phase` builds the 31 vectors in `core/vectors/time.moon.phase.jsonl`. They do not carry the instants: at a phase instant the lit fraction of the disc is known from the phase itself, with no further source needed, so each vector asks for the fraction and the phase name **at** one of USNO's instants. That tests the illumination against USNO's own timing rather than against this code.

## Invariants

- `core/crates/gp-time/tests/moon.rs` `a_full_moon_is_full` checks that at each of USNO's 52 full-moon instants the tool reports over 99% lit and names the phase `full-moon`.
- The vectors check the same at every new moon (all but nothing lit) and every quarter (half lit), which together pin the illuminated fraction at the four points where it is known independently.
