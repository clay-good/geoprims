## Purpose

Provides the coordinate geometry surveyors compute daily (inverses, traverses and their adjustment, intersections, resection, and areas) with exact results and a documented audit trail.

## ADDED Requirements

### Requirement: Bearings, azimuths, and angle notation
COGO tools SHALL accept and output directions as quadrant bearings (`N 45°30'15" E`), north azimuths (0–360°), and optionally south azimuths (labeled), in degrees-minutes-seconds or decimal degrees or gons. Every result SHALL state the reference (grid, geodetic, or assumed) and the linear unit.

#### Scenario: Quadrant bearing parse
- **WHEN** a direction `S 44°30'00" W` is entered
- **THEN** it is read as azimuth 224°30'00"

#### Scenario: Invalid quadrant bearing
- **WHEN** a direction `N 95°00'00" E` is entered
- **THEN** the result is `INVALID_INPUT`, because quadrant bearing angles cannot exceed 90°

### Requirement: Inverse and forward (radial) computations
The inverse tool SHALL compute direction and horizontal distance between two coordinate pairs (northing, easting). The forward tool SHALL compute coordinates from a start point, direction, and distance, including multiple radial sideshots from one station.

#### Scenario: Inverse
- **WHEN** the inverse from (N 1,000.000, E 1,000.000) to (N 1,100.000, E 1,100.000) is computed
- **THEN** the bearing is N 45°00'00" E and the distance is 141.421 (in the input unit)

### Requirement: Traverse closure
Given a closed loop or a traverse connecting two known points, with courses as direction and distance (or as measured angles with a starting azimuth), the tool SHALL compute:
- latitudes and departures
- the angular misclosure against (n − 2) × 180° for interior angles of a loop, or the known closing azimuth
- the linear misclosure √(ΣLat² + ΣDep²) and its direction
- the total length and precision ratio 1 : (length / misclosure)

It SHALL compare these against user-selected precision standards.

#### Scenario: Loop closure
- **WHEN** a loop of four courses (azimuth 0° 300.00, 90° 400.02, 180° 299.95, 270.01° 400.00) is closed
- **THEN** ΣLat ≈ +0.1198, ΣDep ≈ +0.0200, misclosure ≈ 0.1215, length = 1,399.97, and precision ≈ 1:11,525

#### Scenario: Angular misclosure
- **WHEN** interior angles of a 5-sided loop sum to 540°00'25"
- **THEN** the angular misclosure is +25", reported with the allowable misclosure for the chosen standard (e.g. K·√n)

### Requirement: Traverse adjustments
The tool SHALL adjust a traverse by the compass (Bowditch) rule, the transit rule, and the Crandall rule, selectable. It SHALL report per-course corrections, adjusted latitudes and departures, adjusted coordinates, and adjusted directions and distances. The adjusted traverse SHALL close exactly (to 1e-9 of the unit).

#### Scenario: Bowditch correction
- **WHEN** the loop above is adjusted by the compass rule
- **THEN** each course's latitude correction equals −ΣLat × (course length / total length), and the adjusted traverse closes to within 1e-9

#### Scenario: Transit rule depends on orientation
- **WHEN** the transit rule is selected
- **THEN** the result notes that transit-rule corrections depend on the coordinate orientation

### Requirement: Least-squares adjustment (experimental)
An experimental tool SHALL perform a weighted least-squares adjustment of a small 2D network (up to 200 unknown points) from angles, directions, and distances with standard deviations. It SHALL report adjusted coordinates, residuals, standard errors, error ellipses, the reference variance, and a chi-square test result.

#### Scenario: Chi-square failure
- **WHEN** the a-posteriori reference variance fails the chi-square test at 95%
- **THEN** the result flags `ADJUSTMENT_TEST_FAILED` and lists the largest standardized residuals

### Requirement: Intersections and resection
Tools SHALL compute bearing-bearing, bearing-distance (both solutions), and distance-distance (both solutions) intersections, and three-point resection (Tienstra or equivalent). They SHALL detect the danger circle (resection unsolvable or unstable when the station lies on the circle through the three known points) and parallel or non-intersecting cases.

#### Scenario: Two solutions
- **WHEN** a distance-distance intersection has two solutions
- **THEN** both are returned, labeled left and right of the baseline

#### Scenario: Danger circle
- **WHEN** the unknown station is near the circle through the three known points
- **THEN** the result flags `RESECTION_UNSTABLE`

### Requirement: Area by coordinates
The tool SHALL compute area by the coordinate (shoelace) method for plane survey coordinates, in square units, acres (international or US survey acres per the unit), and hectares, and SHALL refer geographic input to the geodesic area tool.

#### Scenario: Rectangle area
- **WHEN** coordinates (0,0), (100,0), (100,50), (0,50) in international feet are entered
- **THEN** the area is 5,000 ft² ≈ 0.1148 acres

### Requirement: Units and grid/ground context
COGO inputs SHALL carry a linear unit (m, ft, ftUS, chains) and a coordinate context (grid with a CRS, or ground with a combined factor). Mixing ft and ftUS in one computation SHALL be rejected unless one is converted explicitly.

#### Scenario: Mixed feet rejected
- **WHEN** a traverse has one course in ftUS and others in ft
- **THEN** the result is `UNIT_MISMATCH` explaining the 2 ppm difference

### Requirement: Calculation sheets and sketches
COGO tools SHALL export a calculation sheet (per io-formats) and SHALL draw a traverse sketch with course labels, the misclosure vector (exaggerated with a scale note), and the adjusted figure.

#### Scenario: Misclosure exaggeration
- **WHEN** the misclosure is drawn
- **THEN** the vector is exaggerated with the factor labeled
