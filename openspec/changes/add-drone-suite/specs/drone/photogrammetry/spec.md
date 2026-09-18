## Purpose

Computes the camera geometry of aerial mapping (ground sampling distance, footprints, overlaps, trigger timing, line spacing, and motion blur) and applies mapping accuracy standards, independent of any vendor or app.

## ADDED Requirements

### Requirement: Ground sampling distance
The GSD tool SHALL compute GSD = (sensor width × height above ground) / (focal length × image width in pixels). It SHALL use the physical focal length and physical sensor dimensions, and SHALL compute both the across-track and along-track GSD when pixels are not square. It SHALL accept a 35 mm-equivalent focal length only when it is labeled as equivalent and a crop factor (or sensor size) is given, converting it to the physical value, and SHALL warn `EQUIVALENT_FOCAL_LENGTH` when the focal length exceeds 1.5× the sensor diagonal (a diagonal field of view under about 37°, unusual for mapping cameras and typical of a 35 mm-equivalent value entered by mistake).

#### Scenario: 1-inch sensor at 100 m
- **WHEN** sensor width = 13.2 mm, focal length = 8.8 mm, image width = 5,472 px, height = 100 m AGL
- **THEN** GSD ≈ 2.741 cm/px and the footprint is 150.0 m across track

#### Scenario: Equivalent focal length suspected
- **WHEN** focal length = 24 mm is entered with a 13.2 × 8.8 mm sensor (diagonal 15.86 mm; threshold 23.80 mm)
- **THEN** the result includes `EQUIVALENT_FOCAL_LENGTH` and asks the user to confirm or convert

### Requirement: Inverse GSD (altitude for a target GSD)
The tool SHALL compute the height above ground required to achieve a target GSD and SHALL flag heights above a user-selected regulatory ceiling (default 400 ft AGL, per operations-reference).

#### Scenario: 2 cm target
- **WHEN** the target GSD is 2.0 cm/px with the same camera
- **THEN** the required height ≈ 72.96 m AGL

### Requirement: Footprint, overlap, trigger, and line spacing
Given camera, orientation (landscape or portrait relative to track), height above ground, front and side overlap, and groundspeed, the tool SHALL compute along- and across-track footprint, trigger distance = along-track footprint × (1 − front overlap), trigger interval = distance / groundspeed, and line spacing = across-track footprint × (1 − side overlap). It SHALL warn when the trigger interval is below a user-entered camera minimum interval (`TRIGGER_TOO_FAST`).

#### Scenario: 75/65 overlap at 10 m/s
- **WHEN** the 1-inch camera flies at 100 m AGL in landscape, 75% front and 65% side overlap, 10 m/s
- **THEN** trigger distance = 25.0 m, trigger interval = 2.5 s, and line spacing = 52.5 m

#### Scenario: Camera cannot keep up
- **WHEN** the camera's minimum interval is 3 s and the computed interval is 2.5 s
- **THEN** the result includes `TRIGGER_TOO_FAST` and the maximum groundspeed that satisfies the camera (≈ 8.33 m/s)

### Requirement: Overlap recommendations are cited guidance
The tool SHALL offer overlap presets with citations: general mapping at least 75% front and 60% side, and forests or dense vegetation at least 85% front and 70% side (Pix4D guidance). They SHALL be labeled as vendor guidance, not standards.

#### Scenario: Forest preset
- **WHEN** a user selects the forest preset
- **THEN** overlaps are set to 85% and 70% with the citation shown

### Requirement: Terrain-aware overlap
Given a terrain profile or a highest-terrain elevation within the area, the tool SHALL compute the effective height above ground at the highest terrain and the resulting worst-case overlap and GSD. It SHALL warn `OVERLAP_BELOW_TARGET` when terrain reduces overlap below the target.

#### Scenario: Hill reduces overlap
- **WHEN** a mission at 100 m above takeoff crosses a 40 m hill
- **THEN** the effective height is 60 m over the hill, the worst-case front overlap is reported, and `OVERLAP_BELOW_TARGET` is raised if it is below 75%

### Requirement: Motion blur
The tool SHALL compute motion blur in pixels = groundspeed × exposure time / GSD, and the maximum exposure time for a blur limit (default 0.5 px).

#### Scenario: Blur at 1/1000 s
- **WHEN** groundspeed = 10 m/s, exposure = 1/1000 s, GSD = 2.741 cm
- **THEN** blur ≈ 0.365 px, and the maximum exposure for 0.5 px ≈ 1/730 s

### Requirement: Oblique imagery GSD
For a camera pitched off nadir by θ, the tool SHALL compute GSD at the image center, near edge, and far edge, and the trapezoidal footprint, and SHALL flag views where the far edge reaches the horizon.

#### Scenario: 45° oblique
- **WHEN** pitch = 45° at 100 m AGL
- **THEN** the center GSD is larger than the nadir GSD, and the footprint is a trapezoid drawn on the map

### Requirement: Image count and survey size
Given an area polygon, the tool SHALL estimate image count, number of flight lines, total line length, and flight time (using the mission-patterns grid). The estimate SHALL agree with the generated pattern within 2%.

#### Scenario: Estimate matches pattern
- **WHEN** the estimate and the generated grid are computed for the same polygon and settings
- **THEN** image counts differ by at most 2%

### Requirement: ASPRS Positional Accuracy Standards (Edition 2)
A tool SHALL implement the ASPRS Positional Accuracy Standards for Digital Geospatial Data, Edition 2 (v2, 2024):
- RMSE-based horizontal accuracy class RMSE_H = √(RMSEx² + RMSEy²), and vertical NVA (pass/fail) and VVA (reported only).
- Product accuracy including checkpoint error: √(RMSE_fit² + RMSE_survey²).
- A minimum of 30 checkpoints, and checkpoint accuracy at least 2× better than the product.
- Blunder threshold of 3 × target RMSE, and mean error expected under 25% of target RMSE.

The tool SHALL NOT report 95% confidence values as the standard's accuracy measure.

#### Scenario: Checkpoint error included
- **WHEN** fit RMSE = 1.00 cm and checkpoint survey RMSE = 2.0 cm
- **THEN** the product accuracy ≈ 2.24 cm

#### Scenario: Too few checkpoints
- **WHEN** 20 checkpoints are supplied
- **THEN** the result includes `INSUFFICIENT_CHECKPOINTS` citing the Edition 2 minimum of 30

#### Scenario: Blunder detection
- **WHEN** one checkpoint error exceeds 3 × the target RMSE
- **THEN** it is flagged as a blunder and listed with its value
