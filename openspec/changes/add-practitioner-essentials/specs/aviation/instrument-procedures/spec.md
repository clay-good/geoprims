## Purpose

Provides the instrument-flying geometry that IFR students, CFIIs, and pilots work through by hand: holding entries, DME math, descent and visual-descent-point planning, approach-light angles, and NOTAM/TFR shapes. All of it is planning and training aid, never navigation.

## ADDED Requirements

### Requirement: Holding entry
Given the holding course (inbound, magnetic), turn direction (standard right or non-standard left), and the aircraft's heading or course to the fix, the tool SHALL determine the recommended entry: direct (180° sector), teardrop (70° sector), or parallel (110° sector), per AIM 5-3-8. When within ±5° of a sector boundary it SHALL report "on the boundary: either entry is acceptable". It SHALL draw the hold with sectors and the aircraft's arrival track.

#### Scenario: Arriving along the inbound course
- **WHEN** the aircraft's course to the fix equals the inbound holding course
- **THEN** the entry is direct

#### Scenario: Left turns mirror
- **WHEN** the same geometry is evaluated with left turns
- **THEN** the teardrop and parallel sectors are mirrored about the inbound course

### Requirement: Holding timing and wind correction
Given TAS, altitude, wind, and leg time or length, the tool SHALL compute:
- the outbound heading with the wind correction applied (with the "triple the drift" rule shown and labeled as a rule of thumb)
- outbound leg timing adjusted to achieve the target inbound leg time
- the applicable maximum holding airspeed from dated reference data (AIM table), flagging when TAS-derived IAS exceeds it

#### Scenario: Speed limit flag
- **WHEN** the planned holding IAS exceeds the maximum for the altitude
- **THEN** the result includes `ABOVE_MAX_HOLDING_SPEED` with the cited limit

### Requirement: DME geometry
Tools SHALL compute:
- ground distance from DME slant range and height above the station: √(DME² − h²), with h in NM, returning `NO_SOLUTION` when h > DME
- DME-arc lead radial for a turn radius and intercept angle
- time and distance to station from a timed bearing change (60 × minutes ÷ degrees, exactly and by the rule)
- radial intercept angles

#### Scenario: Slant range
- **WHEN** DME reads 5.0 NM at 6,000 ft above the station
- **THEN** the ground distance ≈ 4.902 NM (±0.001 NM)

#### Scenario: Overhead
- **WHEN** DME reads 0.8 NM at 6,000 ft above the station
- **THEN** the result is `NO_SOLUTION` with the explanation that the aircraft is essentially overhead

### Requirement: Descent, VDP, and approach geometry
Tools SHALL compute:
- the glidepath vertical speed for a path angle and groundspeed: GS × 101.27 × tan θ fpm (showing the ×5 rule)
- VDP distance from the threshold for a height above threshold and path angle: HAT ÷ tan θ (showing HAT/300 and HAT/318 rules)
- TCH-based path geometry
- height on a VASI/PAPI path at a distance

#### Scenario: Glidepath vertical speed
- **WHEN** θ = 3° and groundspeed = 120 kt
- **THEN** the vertical speed ≈ 637 fpm (rule of thumb: 600 fpm)

#### Scenario: VDP
- **WHEN** HAT = 400 ft and θ = 3°
- **THEN** the VDP is ≈ 1.26 NM from the threshold (HAT/300 rule: 1.33 NM)

### Requirement: Station magnetic variation
Radial-based tools SHALL use the station's published magnetic variation entered by the user, not the current WMM value. They SHALL warn `STATION_VARIATION_DIFFERS` when the two differ by more than 1°.

#### Scenario: Variation mismatch
- **WHEN** a VOR's entered variation is 11° E and WMM gives 8.7° E
- **THEN** the result uses 11° E and warns about the 2.3° difference

### Requirement: NOTAM and TFR geometry
Tools SHALL build the areas in NOTAM and TFR text as polygons on the map, with area and bounds:
- circles from packed coordinates plus radius in NM (e.g. `393400N1224330W` with 3 NM)
- fix-radial-distance references (e.g. `ABC012098.7`), with the user supplying the navaid's coordinates and variation
- point-list polygons

Altitude bounds (SFC to FL or MSL values) SHALL be carried as properties.

#### Scenario: TFR circle
- **WHEN** a user enters `393400N1224330W` radius 3 NM, SFC–3,000 ft MSL
- **THEN** a geodesic circle of 3 NM is drawn and exportable as GeoJSON with its altitude limits
