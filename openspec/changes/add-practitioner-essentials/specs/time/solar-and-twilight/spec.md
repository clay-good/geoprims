## Purpose

Answers the daily sun questions that pilots, drone operators, photogrammetrists, and surveyors ask: when the sun rises and sets, when each kind of twilight and each legal "night" begins, where the sun is, and when the light is right for mapping.

## ADDED Requirements

### Requirement: Solar position
The solar-position tool SHALL compute topocentric solar azimuth, zenith/elevation (with and without refraction), declination, equation of time, and hour angle for a location, height, and instant. It SHALL use the NREL Solar Position Algorithm (±0.0003°, valid −2000 to 6000). It SHALL show the NOAA low-accuracy result alongside as a cross-check. ΔT SHALL come from a dated table with a user override. DUT1 SHALL be an optional input for survey-grade azimuth, with a note that ignoring it (UT1 ≈ UTC) costs up to about 0.004° of azimuth.

#### Scenario: NREL test point
- **WHEN** the NREL SPA publication's example input is computed
- **THEN** zenith and azimuth match the published example values within 1e-5° (their printed precision) and NREL's reference C implementation within 1e-9°

#### Scenario: Survey azimuth note
- **WHEN** solar azimuth is requested with survey precision and no DUT1
- **THEN** the result includes warning `UT1_APPROXIMATED` with the bound

### Requirement: Rise, set, and twilight events
For a location, local date, and explicit UTC offset or named time zone, the tool SHALL return:
- sunrise and sunset, using −0.833° apparent altitude (refraction plus semidiameter), with an optional horizon-dip correction for observer height
- civil (−6°), nautical (−12°), and astronomical (−18°) twilight begin and end
- solar noon and day length

Events SHALL be tied to the requested local date and shown in both local time and UTC (Zulu). Where an event does not occur, the tool SHALL return an explicit state (`polar-day`, `polar-night`, `no-civil-twilight-end`, and so on), never a missing value or NaN.

#### Scenario: Polar night
- **WHEN** sunrise is requested for Utqiaġvik, Alaska (71.29° N, 156.79° W) on December 21
- **THEN** the result is state `polar-night` with the civil twilight window reported if it occurs

#### Scenario: Evening event after 00:00Z
- **WHEN** civil twilight end is requested for Denver on a summer date
- **THEN** the result gives the local time and a Zulu time on the next UTC date, both clearly dated

#### Scenario: NOAA agreement
- **WHEN** 1,000 random locations and dates below 60° latitude are computed
- **THEN** sunrise and sunset agree with the NOAA algorithm within 1 minute

### Requirement: The four US aviation "nights"
The night tool SHALL compute all four time windows for a date and place and label each with its regulation:
- **(a) Logging night (14 CFR 1.1):** end of evening civil twilight to beginning of morning civil twilight, as published in the Air Almanac. Implemented as solar −6°, with the approximation disclosed.
- **(b) Passenger-carrying currency (14 CFR 61.57(b)):** 1 hour after sunset to 1 hour before sunrise.
- **(c) Position lights required (14 CFR 91.209):** sunset to sunrise.
- **(d) Small UAS civil-twilight periods (14 CFR 107.29(c)):** outside Alaska, the 30 minutes before official sunrise and after official sunset; in Alaska, the Air Almanac period. These are twilight periods (during which lighting is required), not a separate night; §1.1 night still applies to Part 107 night operations.

It SHALL never present a single undifferentiated "night". The rule text SHALL come from dated reference data.

#### Scenario: Four windows shown
- **WHEN** a pilot requests nights for a date and airport location
- **THEN** four labeled windows appear, each with its regulation and a one-line meaning ("Night landings for passenger currency count after 21:14 local")

#### Scenario: Landing between definitions
- **WHEN** a landing time is 20 minutes after civil twilight ends and 40 minutes after sunset
- **THEN** the tool states it is loggable night (§1.1) but does not count toward §61.57(b) currency

### Requirement: Night currency counter
Given a list of dated night takeoffs and full-stop landings with times, places, and aircraft category, class, and type (when a type rating is required), entered by the user, the tool SHALL determine, per category/class/type, which count toward §61.57(b), count those within the preceding 90 days of a chosen date, and report whether 3 takeoffs and 3 full-stop landings are met, and the date currency lapses.

#### Scenario: Currency lapse date
- **WHEN** the three qualifying landings are on May 1, May 10, and June 2
- **THEN** for that category and class the tool reports passenger-carrying night currency through July 30, 2026 (the 90th day after the oldest qualifying landing, per the counting convention in the dated reference data, CFI-reviewed), labeled with the citation

### Requirement: Photogrammetry and field lighting
Tools SHALL compute:
- the time windows when solar elevation is above a user threshold (default 30°, cited as common vendor guidance)
- the hotspot condition (sun directly behind the camera look vector) for a given camera pitch and heading
- shadow length for an object height
- solar incidence angle on a slope given slope and aspect (from the terrain tools, or typed)

#### Scenario: Mapping window
- **WHEN** a mapping date and site are entered with threshold 30°
- **THEN** the tool returns the local and UTC window, and the canvas plots solar elevation through the day with the window shaded

### Requirement: Sun path visualization
Solar tools SHALL draw a sun path diagram (azimuth vs elevation through the day, with the current instant marked) and, on the map, the sun azimuth and shadow direction at the site.

#### Scenario: Sun path
- **WHEN** a date and site are set
- **THEN** the sun-path diagram shows the day's arc with rise, noon, and set labeled
