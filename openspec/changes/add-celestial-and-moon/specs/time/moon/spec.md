## Purpose

Answers the moon questions of night operations and planning: where it is, when it rises and sets, and how bright it is.

## ADDED Requirements

### Requirement: Moon position
The tool SHALL return the moon's topocentric azimuth and altitude (with and without refraction), geocentric RA and Dec, distance, horizontal parallax, and semidiameter for a place and UT instant, within 0.2' of USNO.

#### Scenario: Position parity
- **WHEN** 100 place-instant pairs are compared with USNO
- **THEN** azimuth and altitude agree within 0.2'

### Requirement: Moonrise, moonset, and transit
For a local date and zone (explicit offset or tzdb name), the tool SHALL return moonrise, moonset, and upper transit by the USNO definition (geocentric zenith distance of the center 90°34' + SD − HP, upper limb with 34' refraction), in local time and UTC, each dated, with the explicit state `POLAR_MOON` when an event does not occur that day and a note when the moon rises or sets twice or not at all because of its daily retardation. Times SHALL agree with USNO within 1 minute.

#### Scenario: USNO parity
- **WHEN** 500 place-date pairs are compared with USNO rise, set, and transit times
- **THEN** every time agrees within 1 minute

#### Scenario: No moonset today
- **WHEN** the moon sets just before midnight on the previous day and after midnight on the next at the place
- **THEN** the result states that there is no moonset on the requested local date, with the previous and next set times

### Requirement: Phase and illumination
The tool SHALL return the illuminated fraction (0 to 1, to 0.001), phase angle, the moon's age (days since new), the phase name, and the instants of the next new moon, first quarter, full moon, and last quarter, within 2 minutes of USNO.

#### Scenario: Full moon
- **WHEN** the next full moon after a date is requested
- **THEN** its instant matches USNO within 2 minutes and the illuminated fraction at that instant exceeds 0.99

### Requirement: Moon on the sun-path diagram
When a sun-path diagram is shown for the date, the moon's path and its rise and set SHALL be drawn on it in a second style, from the core's samples.

#### Scenario: Both paths
- **WHEN** the moonrise tool runs for a site and date
- **THEN** the sky plot shows the sun's and the moon's paths with rise and set labeled for each
