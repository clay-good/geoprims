## Purpose

Adds the time scales and calendar forms that astronomy, GNSS, and software use: sidereal times, Unix time, and ISO week dates.

## ADDED Requirements

### Requirement: Sidereal time
The sidereal tool SHALL return GMST, GAST, ERA, the equation of the equinoxes, and, for a longitude, LMST and LAST, by IAU 2006/2000A (Capitaine, Wallace & Chapront 2005; IERS Conventions 2010 chapter 5), from UT1 (UTC with DUT1, warning `UT1_APPROXIMATED` without it) and TT (via ΔT). It SHALL show Meeus's IAU 1982 GMST beside the result with the difference. Values SHALL agree with ERFA within 1e-9 rad and with USNO within 0.001 s.

#### Scenario: J2000.0
- **WHEN** 2000-01-01T12:00:00 UT1 is converted
- **THEN** GMST is 18h 41m 50.548s (±0.001 s) and ERA is 280.4606° (±0.0001°)

#### Scenario: Local sidereal
- **WHEN** longitude 74°30'W is given
- **THEN** LMST = GMST − 4h 58m 00s exactly

### Requirement: Unix time
The tool SHALL convert Unix seconds ↔ UTC by the POSIX.1-2024 §4.19 expression (every day 86,400 s; leap seconds not counted, stated in the result), accepting seconds, milliseconds, microseconds, and nanoseconds with the magnitude detected from the digit count and reported with `INPUT_NORMALIZED`, and SHALL also return the ISO 8601 UTC string, the Julian date, and the GPS week and seconds (by the existing tools' rules).

#### Scenario: Epoch
- **WHEN** 0 is converted
- **THEN** the result is 1970-01-01T00:00:00Z

#### Scenario: Milliseconds detected
- **WHEN** 1760832000000 is entered
- **THEN** it is read as milliseconds (2025-10-19T00:00:00Z) with `INPUT_NORMALIZED` naming the unit assumed

Spec note (2026-10-20): the scenario first said 2026-10-19; 1,760,832,000 s is 20,380 days after the epoch, 2025-10-19, as Python's datetime agrees (2026-10-19 would be 1792368000).

#### Scenario: Leap second instant
- **WHEN** 2016-12-31T23:59:60Z is entered
- **THEN** the result is `INVALID_INPUT` explaining that Unix time has no representation for a leap second, with the hint to use TAI or GPS time

### Requirement: ISO week dates
The tool SHALL convert a calendar date ↔ ISO week date (ISO 8601-1:2019: weeks start on Monday; week 1 contains January 4; a year has 53 weeks when it starts on a Thursday, or on a Wednesday in a leap year), returning the week-year, week, and weekday, and the week's Monday and Sunday.

#### Scenario: Year boundary
- **WHEN** 2027-01-01 is converted
- **THEN** the ISO week date is 2026-W53-5

#### Scenario: Week 1
- **WHEN** 2026-01-01 is converted
- **THEN** the ISO week date is 2026-W01-4
