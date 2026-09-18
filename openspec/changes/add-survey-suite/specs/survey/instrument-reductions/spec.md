## Purpose

Reduces raw total-station, level, and EDM observations to horizontal distances, elevation differences, and grid quantities. Every correction and constant is named, so results can be checked and audited.

## ADDED Requirements

### Requirement: Slope reduction
Given slope distance and a zenith angle (or vertical angle, declared), the tool SHALL compute horizontal distance HD = SD·sin Z, vertical difference VD = SD·cos Z, and elevation difference ΔElev = HI + VD − HR. Face-left/face-right zenith pairs SHALL be accepted and the mean and index error computed.

#### Scenario: 500 m at 85°
- **WHEN** SD = 500.000 m and Z = 85°00'00"
- **THEN** HD ≈ 498.097 m and VD ≈ 43.578 m

#### Scenario: Two-face observation
- **WHEN** face-left Z = 85°00'10" and face-right Z = 274°59'40" are entered
- **THEN** the mean zenith is 85°00'15" and the index error is −5" (FL + FR = 359°59'50"; index error = (FL + FR − 360°)/2, convention stated)

### Requirement: Curvature and refraction
The tool SHALL compute the combined curvature-and-refraction correction h = (1 − k)·D²/(2R) with the refraction coefficient k (default 0.13, editable) and R (default 6,371,000 m) shown. It SHALL display the matching textbook coefficient for the chosen k (0.0675 m/km² corresponds to k ≈ 0.14, and 0.0683 m/km² to k = 0.13), and apply the correction to trigonometric leveling when the sight exceeds a user threshold.

#### Scenario: 1 km sight
- **WHEN** D = 1 km with k = 0.14
- **THEN** the correction ≈ 0.0675 m; with k = 0.13 it is ≈ 0.0683 m, and both coefficients are labeled with their k

### Requirement: EDM atmospheric correction
The tool SHALL compute the EDM first-velocity correction in ppm from temperature, pressure, and humidity using the instrument's reference refractive index (user-entered, or presets labeled by manufacturer formula), and SHALL apply it to a measured distance. Prism constant SHALL be a separate input.

#### Scenario: ppm applied
- **WHEN** a distance of 1,000.000 m is corrected with +12 ppm and a prism constant of -30 mm
- **THEN** the corrected distance is 1,000.000 + 0.012 − 0.030 = 999.982 m, with each correction listed

### Requirement: Grid ↔ ground scale factors
The tool SHALL compute the elevation factor EF = R / (R + h), where h is the ellipsoid height. It SHALL require or derive h from orthometric height plus a geoid model (via geodesy heights) and SHALL refuse to use orthometric height as h without warning `ORTHOMETRIC_AS_ELLIPSOIDAL`. It SHALL also compute the grid scale factor (from geodesy projections), the combined factor CF = k × EF, and conversions of distances between grid and ground, including project-average combined factors.

#### Scenario: Combined factor
- **WHEN** grid scale factor k = 0.99990 and ellipsoid height h = 1,500 m with R = 6,371,000 m
- **THEN** EF ≈ 0.99976461 and CF ≈ 0.99966464

#### Scenario: Orthometric height guard
- **WHEN** a user enters only an orthometric height and no geoid model
- **THEN** the result includes `ORTHOMETRIC_AS_ELLIPSOIDAL` with the typical magnitude of the error

### Requirement: Differential leveling
Given a level-run table (backsights, foresights, and optionally intermediate sights), the tool SHALL compute heights of instrument and elevations. For closed loops or runs between benchmarks, it SHALL compute the misclosure and compare it with allowable closure (e.g. constant × √distance for a chosen order and class). It SHALL distribute the misclosure proportionally to distance or setups, and SHALL check arithmetically (ΣBS − ΣFS = last − first elevation).

#### Scenario: Arithmetic check
- **WHEN** a level run is reduced
- **THEN** the result includes the arithmetic check and fails loudly if it does not balance

### Requirement: Stadia and trigonometric heights
Tools SHALL reduce stadia readings (interval × stadia constant, inclined sights) and compute heights of inaccessible objects (from two stations or one station with a measured baseline).

#### Scenario: Inaccessible height
- **WHEN** zenith angles to the base and top of a tower are measured with a horizontal distance
- **THEN** the tower height is computed, with curvature and refraction applied if the distance exceeds the threshold

### Requirement: Total-station offsets
Tools SHALL compute coordinates for distance-offset shots (left/right, in/out, up/down) and angle-offset shots (for a point whose center cannot hold a prism).

#### Scenario: Angle offset for a tree center
- **WHEN** a distance is measured to the side of a tree and the angle is turned to its center
- **THEN** the center's coordinates use the measured distance plus the entered radius along the center direction
