## Purpose

Converts between the time scales and formats that aviation, GNSS, surveying, and astronomy use (local, UTC/Zulu, GPS, TAI, Julian dates, decimal hours), without depending on the browser's clock or time-zone database.

## ADDED Requirements

### Requirement: Explicit time zones, deterministic data
Local↔UTC conversions SHALL take either an explicit UTC offset or a named IANA zone resolved against a versioned tzdb snapshot bundled as a data asset (the version is echoed in `meta.assets`). Tools SHALL NOT use the host's `Intl` time-zone data, preserving determinism. Ambiguous or nonexistent local times (DST fall-back and spring-forward) SHALL be reported explicitly with both candidates or `INVALID_INPUT`.

#### Scenario: DST gap
- **WHEN** 02:30 local on the US spring-forward date in America/Denver is converted
- **THEN** the result is `INVALID_INPUT` stating that the local time does not exist that day

#### Scenario: Zulu format
- **WHEN** 14:05 local in America/Chicago (CDT) is converted
- **THEN** the result reads `1905Z` and the ISO 8601 UTC timestamp

### Requirement: Decimal hours and logbook arithmetic
Tools SHALL convert decimal hours ↔ h:mm (1.3 h = 1:18, never 1:30), add and subtract durations, and compute block time from out/in times across midnight and across UTC dates.

#### Scenario: Decimal hours
- **WHEN** 1.3 hours is converted
- **THEN** the result is 1 h 18 min

### Requirement: GNSS and astronomical time
Tools SHALL convert UTC ↔ GPS week and seconds-of-week, including the 1,024-week rollover. A 10-bit week number SHALL require an era, and without one SHALL return `INVALID_INPUT`. Tools SHALL also convert GPS ↔ TAI ↔ UTC using a leap-second table from IERS Bulletin C (a data asset with its bulletin number), UTC ↔ Julian date, MJD, and day of year (including day 366), and GPS day-of-year file naming for RINEX and OPUS.

#### Scenario: GPS week
- **WHEN** 2026-09-18T00:00:00Z is converted
- **THEN** GPS week = 2436 and seconds of week = 432018 (with GPS − UTC = 18 s)

#### Scenario: Julian date
- **WHEN** 2026-09-18T00:00:00Z is converted
- **THEN** JD = 2461301.5, MJD = 61301, and day of year = 261

#### Scenario: Leap-second table stale
- **WHEN** the date is beyond the leap-second table's validity (per the latest Bulletin C)
- **THEN** the result includes warning `LEAP_SECOND_TABLE_EXPIRED`

### Requirement: Time zone from location (optional data)
When the optional `tz-boundaries` asset is installed, a tool SHALL return the IANA zone for a coordinate with the asset's attribution (ODbL). Without it, the tool SHALL ask for a zone or offset and never guess from longitude.

#### Scenario: No boundary data
- **WHEN** a user asks for the local zone at a coordinate without the asset
- **THEN** the tool offers to download the asset (size shown) or to enter a zone manually
