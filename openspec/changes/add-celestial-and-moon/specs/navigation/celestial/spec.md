## Purpose

Computes the almanac data a navigator reads from tables, reduces a sextant sight to a line of position, corrects a sextant altitude, and checks a compass by the sun, from open series verified against the US Naval Observatory. Nothing from the Nautical Almanac is reproduced.

## ADDED Requirements

### Requirement: Computed almanac data
The almanac tool SHALL return, for one UT instant (with ΔT from the dated table and optional DUT1), GHA and Dec of the sun, moon (with horizontal parallax and semidiameter), Venus, Mars, Jupiter, and Saturn, GHA of Aries, SHA and Dec of the 57 navigational stars and Polaris, and the equation of time. Values SHALL be within 0.1' of the USNO Astronomical Applications API for the sun, planets, Aries, and stars, and within 0.2' for the moon, over 1990–2050. The result SHALL name the series used and state that the Nautical Almanac is the governing publication.

#### Scenario: USNO parity
- **WHEN** 200 instants and 20 positions are compared with USNO celnav output
- **THEN** every GHA and Dec is within its declared tolerance

#### Scenario: Out of range
- **WHEN** the instant is before 1800 or after 2100
- **THEN** the result is `OUT_OF_DOMAIN` naming the validated range

### Requirement: Sight reduction by the intercept method
Given a body, UT, assumed position (or a DR position with the assumed-position rule applied and shown), and observed altitude Ho, the tool SHALL return GHA, Dec, LHA, Hc, Z, Zn (with the quadrant rule stated), the intercept in nautical miles with Toward or Away, and the line of position for the map. Hc SHALL be within 0.1' and Zn within 0.1° of Bowditch's worked examples.

#### Scenario: Deneb sight
- **WHEN** Bowditch §1906's Deneb sight is reduced (2016-03-09 05:58:27 zone +3, DR 39°N 045°26.0'W, Ho 50°25.8')
- **THEN** LHA is 306°, Hc 50°14.1', Zn 063.3°, and the intercept 11.7 nm Toward

#### Scenario: Body below the horizon
- **WHEN** the computed Hc is negative
- **THEN** the result carries `BODY_BELOW_HORIZON` and still reports Hc and Zn

#### Scenario: Observed at the geographic position
- **WHEN** the assumed position equals the body's geographic position and Ho is 90°
- **THEN** Hc is 90° and the intercept is 0 nm

### Requirement: Altitude corrections shown step by step
The altitude-correction tool SHALL convert a sextant altitude to the observed altitude with each step reported: index correction, dip (1.76'√h with h in meters, or 0.97'√h in feet), refraction by Bennett 1982 with the temperature and pressure factor, semidiameter (sun from its computed distance, moon from HP) for the chosen limb, parallax in altitude (moon HP cos ha; sun 0.15'; planets from their HP), and a note below 10° apparent altitude. Agreement with the Nautical Almanac's corrections SHALL be within 0.1'.

#### Scenario: Sun lower limb
- **WHEN** Bowditch §1907's sun sight is corrected (hs 6°37.5', height of eye 68 ft, 88 °F, 982 mb, lower limb)
- **THEN** Ho matches the example within 0.1' and the low-altitude note is shown

### Requirement: Latitude by Polaris
The tool SHALL compute latitude from an observed altitude of Polaris and LHA Aries by the computed polar distance and azimuth (not the Almanac's a₀, a₁, a₂ tables), reporting Polaris's true azimuth as well.

#### Scenario: Polaris example
- **WHEN** Bowditch §1912's Polaris observation is reduced
- **THEN** the latitude matches the example within 0.1'

### Requirement: Sun azimuth and amplitude for a compass check
The sun-azimuth-check tool SHALL return the sun's true azimuth at an instant and place, its amplitude at rising or setting (sin A = sin d / cos L, with the visible-horizon note), and, given a gyro or compass bearing, the gyro error or compass error, with variation from the magnetic tools when a magnetic compass is checked.

#### Scenario: Gyro error
- **WHEN** the sun's computed azimuth is 124.4° and the gyro reads 124.0°
- **THEN** the gyro error is 0.4° W (per Bowditch §1501's sign convention)
