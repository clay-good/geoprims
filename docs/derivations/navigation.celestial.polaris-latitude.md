<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Latitude by Polaris (`navigation.celestial.polaris-latitude`)

## Method

Polaris sits under a degree from the north celestial pole, and the altitude of the elevated pole above the celestial horizon is the observer's latitude. So a corrected altitude of Polaris is very nearly the latitude, and the whole job is the small correction for Polaris not being at the pole.

The Nautical Almanac does that correction with three tabulated terms, a₀, a₁, and a₂, which between them carry the series

    Latitude − Ho = −p cos h + ½ p sin p sin² h tan(latitude)

for a mean position of Polaris at a mean latitude of 50°, with constants added to keep each term positive. This tool does not use those tables. It solves the navigational triangle itself, for Polaris's own apparent place at the second of the sight:

1. Polaris's apparent place comes from the same `gp-astro` series as the almanac tool: Hipparcos (van Leeuwen 2007), with proper motion, parallax, light deflection, and aberration, carried to the true equator and equinox of date by IAU 2006 precession and IAU 2000A nutation. GHA = GAST − α.
2. The local hour angle is the Greenwich hour angle plus the longitude, east positive.
3. The latitude is the one that would see the observed altitude with the body at that declination and hour angle, taken from the altitude equation exactly rather than from a series.
4. Polaris's true azimuth comes from the same triangle, so the line of position can be drawn square to it.

Because the solution is exact, it holds at any latitude and for any polar distance, and it will not drift as Polaris moves off its adopted mean position between editions of the Almanac.

## Equations

- LHA = (GHA + λ) mod 360°, λ east positive.
- The altitude equation, sin Ho = sin L sin d + cos L cos d cos LHA, is linear in sin L and cos L, so it rewrites as one sinusoid in L:
  - A = sin d, B = cos d cos LHA, R = √(A² + B²), φ = atan2(B, A)
  - R sin(L + φ) = sin Ho, so L = asin(sin Ho / R) − φ, or L = 180° − asin(sin Ho / R) − φ
- Of those two roots the tool returns the one nearer the observed altitude, which for a body within a degree of the pole is always the intended one.
- p = 90° − d, the polar distance, reported in minutes of arc.
- Zn = atan2(−cos d sin LHA, cos L sin d − sin L cos d cos LHA), reduced to [0°, 360°).
- The correction the result reports is (L − Ho) in minutes of arc, the same quantity as a₀ + a₁ + a₂ − 1°.

## Symbols and units

L is the latitude, λ the longitude, d Polaris's declination, h or LHA its local hour angle, p its polar distance, Ho the observed altitude, and Zn its true azimuth. Latitude, declination, and hour angles are output in degrees; the correction and the polar distance in minutes of arc. Trigonometry runs in radians.

## Domain

A UTC time from 1800-01-01 to 2100-12-31 (OUT_OF_DOMAIN otherwise); DUT1 within ±0.9 s; an observed altitude from 0° to 90° (INVALID_INPUT otherwise); any longitude. The ratio sin Ho / R is clamped to [−1, 1], which only bites within a thousandth of a degree of the pole.

## Approximations

- **None in the latitude itself.** The root returned satisfies the altitude equation exactly, to double precision. The a₀, a₁, a₂ series is not used, so its mean-position and mean-latitude assumptions do not apply here.
- **Geocentric place:** Polaris's declination and hour angle are geocentric, the almanac convention. The observer's parallax belongs to the altitude correction, and for a star it is nil.
- **Refraction:** none is applied. The observed altitude is already corrected.
- **The root chosen:** the tool takes the root nearer the observed altitude. For Polaris the other root is near 180° − Ho and never plausible, but the rule is a choice, not a proof, so the invariant test below checks the returned latitude against a full sight reduction.
- **One sight, one latitude:** this is a latitude, not a fix. The latitude depends on the longitude through the hour angle, so the longitude used must be a good one.

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: The American Practical Navigator (Bowditch), NGA Pub. No. 9, Volume I
- sourceEdition: 2024 edition
- sourceLocator: Chapter 20, §2015, Latitude by Polaris, the worked morning sight of 2024-06-02
- independent: yes
- inputs: Ho 16°11.3′ at 2024-06-02T12:25:12Z, longitude 110°07.3′W
- outputs: GHA Aries 77°45.9′, correction −8.1′, latitude 16°03.2′N, azimuth 000.8°
- tolerance: 0.2′ in latitude and in the correction, 0.2° in azimuth. The example's a₀, a₁, and a₂ are each rounded to a tenth of a minute before they are added, and its azimuth is read from a table printed to a tenth of a degree, so the printed digits cannot be reproduced closer than that.
- verifiedBy: `core/crates/gp-navigation/tests/celestial.rs` `bowditch_polaris_sight`, and golden vector v001
- verifiedOn: 2026-10-27

The example's own GHA Aries, 71°26.9′ for 12h plus 6°19.0′ for 25m12s, is 77°45.9′, which the tool reproduces exactly. Its printed LHA Aries, 327°38.8′, is 0.2′ away from that figure less the longitude, which is 327°38.6′; the 0.2′ is the book's, not the tool's, and the tolerance above covers it.

## Differential tests

- `tools/vectors/gen_celestial_checks.py navigation.celestial.polaris-latitude` builds the 36 vectors in `core/vectors/navigation.celestial.polaris-latitude.jsonl`: Bowditch's worked sight, 33 rows of USNO's celnav API read backwards, and the out-of-range and near-the-pole scenarios.
- The USNO rows are the tool's real check, and they test Polaris's place and the latitude solver together. Each row gives USNO's computed altitude of Polaris at a known position and instant; the vector feeds that altitude back in as the observed altitude with the row's longitude, and expects the row's latitude. 33 instants and places from 1995 to 2045, latitudes 2°N to 72°N, all within 0.1′ of latitude and 0.05° of azimuth.

## Invariants

- `core/crates/gp-navigation/tests/celestial.rs` `the_polaris_latitude_reproduces_the_sight` checks the solver against the sight-reduction tool at four altitudes and longitudes: reducing a Polaris sight from the latitude returned must give an intercept of zero and the same azimuth. That is the statement that the returned root really is a root, and that the right root was picked.
- `core/crates/gp-navigation/tests/celestial.rs` `bowditch_polaris_sight` also checks that the reported correction is the published a₀ + a₁ + a₂ − 1°, and that the written latitude reads as a work form prints it.
