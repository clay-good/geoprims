## Purpose

Answers the practical drone-mission questions beyond camera geometry: how far the aircraft can be seen, when lighting rules apply, how dense lidar data will be, how big the dataset will be, whether the radio link will hold, and what a thermal camera can resolve.

## ADDED Requirements

### Requirement: Visual line of sight guidance
The VLOS tool SHALL compute:
- attitude line of sight (ALOS) from the aircraft's characteristic dimension using the EASA guidance formulas (multirotor and fixed-wing coefficients, cited)
- detection line of sight (DLOS) from an entered meteorological visibility
- VLOS = min(ALOS, DLOS)

It SHALL compare the result with the farthest point of a planned mission. The result SHALL be labeled "EASA guidance; US Part 107 sets no numeric VLOS distance".

#### Scenario: Small multirotor
- **WHEN** the characteristic dimension is 0.35 m (multirotor)
- **THEN** ALOS ≈ 134 m (327 × 0.35 + 20), labeled with the EASA source

#### Scenario: Mission beyond VLOS
- **WHEN** a mission's farthest waypoint is 400 m from the pilot and VLOS is 134 m
- **THEN** the result reads "Beyond VLOS guidance by 266 m"

### Requirement: Part 107 lighting window
For a date and site, the tool SHALL compute the Part 107 civil-twilight periods (30 minutes before official sunrise and 30 minutes after official sunset outside Alaska; the Air Almanac period in Alaska), state that anti-collision lighting visible for 3 statute miles is required outside daylight, and link to the solar tool.

#### Scenario: Evening window
- **WHEN** sunset is 19:12 local
- **THEN** the civil-twilight period ends 19:42 local, and operations after that are "night" under §107.29 with the lighting requirement stated

### Requirement: Lidar mission planning
Given sensor pulse rate, field of view, number of returns, flight height, speed, and side overlap, the tool SHALL compute:
- swath width (2H·tan(FOV/2))
- line spacing
- nominal pulse density (pulse rate ÷ (speed × swath)), and aggregate density including overlap
- expected point density per return

It SHALL compare pulse density with USGS 3DEP quality levels (QL1 ≥ 8 pulses/m², QL2 ≥ 2 pulses/m², cited and dated). It SHALL distinguish pulses from points, and SHALL state that non-repetitive scan patterns are not uniform across the swath.

#### Scenario: Density at 100 m
- **WHEN** pulse rate = 240,000/s, FOV = 70°, height = 100 m, speed = 10 m/s
- **THEN** the swath ≈ 140.0 m and nominal pulse density ≈ 171 pulses/m², exceeding QL1

### Requirement: Dataset size estimate
Given image count and per-image size (or sensor pixels and format), or a lidar point count, the tool SHALL estimate:
- raw capture size
- orthomosaic size (area ÷ GSD², times bands and bit depth, with a stated compression factor range)
- point cloud size in LAS and LAZ

It SHALL label the result as an estimate with its assumptions.

#### Scenario: Orthomosaic estimate
- **WHEN** a 1 km² area at 2 cm GSD, 3 bands, and 8-bit is estimated
- **THEN** the uncompressed size is 7.5 GB, and a compressed range is shown with the assumed ratio

### Requirement: Radio link budget
The link tool SHALL compute:
- free-space path loss (20·log₁₀(d_km) + 20·log₁₀(f_MHz) + 32.44)
- received power from transmit power, antenna gains, and cable losses
- fade margin against receiver sensitivity

It SHALL combine with Fresnel and terrain line-of-sight checks. Regulatory EIRP limits SHALL be dated reference data by jurisdiction, and the tool SHALL never recommend exceeding them.

#### Scenario: 2.4 GHz at 5 km
- **WHEN** f = 2,400 MHz and d = 5 km
- **THEN** FSPL ≈ 114.0 dB

### Requirement: Thermal pixel footprint
Given the sensor's instantaneous field of view (IFOV, mrad) or pixel pitch and focal length, and distance, the tool SHALL compute pixel footprint (IFOV × distance). It SHALL compute the minimum reliably measurable target size using a stated rule (default 3 × 3 pixels, cited as industry guidance), and the maximum distance for a target size.

#### Scenario: Solar panel inspection
- **WHEN** IFOV = 1.3 mrad at 30 m
- **THEN** the pixel footprint is 39 mm and the minimum measurable target is 117 mm (3 × 3 rule)
