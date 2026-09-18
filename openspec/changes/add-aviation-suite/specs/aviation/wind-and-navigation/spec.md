## Purpose

Replaces the flight computer (E6B) with exact wind-triangle, runway-wind, heading-chain, and off-course calculations, drawn as vector diagrams, with true-versus-magnetic references made explicit.

## ADDED Requirements

### Requirement: Wind triangle, all forms
The wind-triangle tool SHALL solve every standard form:
- **Heading and groundspeed** from true course, TAS, and wind.
- **Wind** from true heading, TAS, track, and groundspeed.
- **TAS and heading** from course, groundspeed, and wind.
- **Course and groundspeed** from heading, TAS, and wind.

Wind direction SHALL be the direction the wind blows FROM. All angles SHALL be declared true or magnetic, and inputs of mixed reference SHALL be rejected unless a variation is supplied. When the crosswind component exceeds TAS, the tool SHALL return `NO_SOLUTION` with an explanation.

#### Scenario: Heading and groundspeed
- **WHEN** true course = 090°, TAS = 120 kt, wind = 030° true at 20 kt
- **THEN** WCA ≈ -8.30° (left), true heading ≈ 081.7°, groundspeed ≈ 108.7 kt

#### Scenario: Find the wind
- **WHEN** true heading = 081.7°, TAS = 120 kt, track = 090°, groundspeed = 108.7 kt
- **THEN** the wind is ≈ 030° true at 20 kt

#### Scenario: Wind too strong
- **WHEN** TAS = 30 kt with a 40 kt direct crosswind
- **THEN** the result is `NO_SOLUTION` stating that the course cannot be held

#### Scenario: Mixed references rejected
- **WHEN** a true course is combined with a magnetic wind (e.g. ATIS) without a variation
- **THEN** the tool returns `INVALID_INPUT` asking for the variation or a consistent reference, and explains that METAR/TAF/winds aloft are true while ATIS/tower winds are magnetic in US practice

### Requirement: Runway wind components and limits
Given runway heading (from a runway designator such as `27`, `09L`, or `36T`, or a precise magnetic or true heading), wind direction, speed, and optional gust, the tool SHALL return headwind/tailwind and left/right crosswind components (steady and gust). It SHALL compare them against user-entered limits (maximum crosswind, maximum tailwind) and report "Within / Near / Beyond your <limit>" with text and icon (per `ux/glanceable-results`).

#### Scenario: Runway 27 with a right crosswind
- **WHEN** runway 27 (270° magnetic), wind 300° magnetic at 15 kt
- **THEN** headwind ≈ 13.0 kt and crosswind ≈ 7.5 kt from the right

#### Scenario: Gust component
- **WHEN** the wind is 300° at 15 gusting 25 kt
- **THEN** the gust crosswind ≈ 12.5 kt is reported alongside the steady component

#### Scenario: Designator heading precision
- **WHEN** runway `27` is entered without a precise heading
- **THEN** the tool uses 270° and warns `RUNWAY_HEADING_APPROXIMATE` (actual runway headings can differ by up to ±5°)

### Requirement: Best runway selection
Given a list of runways and a wind, a tool SHALL rank runways by headwind component, report components for each, and flag runways that exceed the entered limits.

#### Scenario: Ranking
- **WHEN** runways 09/27 and 18/36 are evaluated with wind 200° at 12 kt
- **THEN** runway 18 ranks first, with its components shown, and every runway is listed

### Requirement: Heading chain
A tool SHALL convert through the chain true course → wind correction → true heading → variation → magnetic heading → deviation → compass heading. Variation comes from a model or chart entry (per geomagnetism), and deviation comes from a user-entered compass deviation card, interpolated. Each step is shown.

#### Scenario: Deviation card interpolation
- **WHEN** a deviation card lists +2° at 060° and -1° at 090° and the magnetic heading is 075°
- **THEN** the interpolated deviation is +0.5° and the compass heading is shown

### Requirement: Off-course correction (1-in-60)
Given distance flown, distance off course, and distance remaining, a tool SHALL compute the track-error angle, the correction to parallel the course, and the correction to converge on the destination, both exactly and by the 1-in-60 rule, labeled.

#### Scenario: 4 NM off after 60 NM
- **WHEN** 4 NM off course after 60 NM with 60 NM remaining
- **THEN** exact track error ≈ 3.81° (1-in-60: 4°), and the correction to reach the destination ≈ 7.63° exact (1-in-60: 8°)

### Requirement: Wind diagrams
Wind tools SHALL draw the wind triangle (air vector, wind vector, ground vector) head-to-tail with labeled angles, and runway-wind tools SHALL draw the runway with wind components as arrows.

#### Scenario: Runway diagram
- **WHEN** runway components are computed
- **THEN** the diagram shows the runway, the wind arrow, and the headwind and crosswind component arrows with values

### Requirement: Wind edge cases
Wind inputs SHALL accept calm (`00000KT`, 0 kt) and variable direction (`VRB05KT`, or a variation range such as `280V340`). With calm wind the triangle SHALL return WCA 0 and groundspeed equal to TAS. With variable direction the runway tool SHALL report the worst-case crosswind and tailwind over the range (or over all directions for `VRB`). TAS of 0 SHALL be rejected with `INVALID_INPUT`. Deviation-card and wind-direction interpolation SHALL wrap across 360°/000°.

#### Scenario: Variable wind worst case
- **WHEN** runway 27 is evaluated with wind `VRB08KT`
- **THEN** the worst-case crosswind is 8 kt and the worst-case tailwind is 8 kt, both labeled worst case

#### Scenario: Deviation card across north
- **WHEN** a deviation card lists −1° at 330° and +1° at 030° and the heading is 000°
- **THEN** the interpolated deviation is 0°
