## Purpose

Solves distance, direction, and position problems on the Earth ellipsoid to reference accuracy, and exposes the common approximations (Vincenty, spherical great circle, rhumb line) with their errors made explicit.

## ADDED Requirements

### Requirement: Geodesic inverse (default method)
The inverse tool SHALL compute, for two points on a chosen ellipsoid (default WGS 84), the geodesic distance s12, initial azimuth azi1, final azimuth azi2, and optionally reduced length m12, geodesic scale M12/M21, and area under the geodesic S12, using Karney's algorithm (2013). It SHALL converge for all point pairs, including coincident, polar, and antipodal pairs, with accuracy of 15 nm (nanometers) or better on WGS 84.

#### Scenario: JFK to LHR
- **WHEN** the inverse is computed from (40.6413°, -73.7781°) to (51.4700°, -0.4543°) on WGS 84
- **THEN** s12 ≈ 5,554,908.791 m (2,999.411 NM), azi1 ≈ 51.3816479°, azi2 ≈ 107.9828291° (±1 mm, ±1e-7°)

#### Scenario: Nearly antipodal
- **WHEN** the inverse is computed from (0°, 0°) to (0.5°, 179.7°)
- **THEN** s12 ≈ 19,944,127.421 m (±1 mm) and azi1 ≈ 15.556883° (±1e-6°), with no convergence warning

#### Scenario: Exactly antipodal
- **WHEN** the inverse is computed between (0°, 0°) and (0°, 180°)
- **THEN** the distance is returned, and the result includes warning `AZIMUTH_NOT_UNIQUE` stating that infinitely many geodesics connect the points

#### Scenario: Coincident points
- **WHEN** both points are identical
- **THEN** s12 is 0, and azimuths are returned with warning `AZIMUTH_UNDEFINED`

### Requirement: Geodesic direct
The direct tool SHALL compute the destination point and final azimuth from a start point, initial azimuth, and distance (positive or negative, any length including multiple circumnavigations), with the same accuracy.

#### Scenario: Direct 1,000 km
- **WHEN** the direct problem is solved from (40.6413°, -73.7781°) with azimuth 51° for 1,000,000 m
- **THEN** the destination is ≈ (45.8920808°, -63.7549563°) (±1e-7°) and the final azimuth ≈ 57.886373° (±1e-6°)

### Requirement: Vincenty methods are available and honest
Vincenty direct and inverse SHALL be available as separate tools, labeled as legacy methods, with a declared convergence tolerance (1e-12 rad) and iteration limit (200). On non-convergence the inverse SHALL return `DID_NOT_CONVERGE` with a hint to the default method. Each Vincenty result SHALL also report the difference from the Karney result for the same inputs.

#### Scenario: Vincenty non-convergence
- **WHEN** the Vincenty inverse is run from (0°, 0°) to (0.5°, 179.7°)
- **THEN** it returns `DID_NOT_CONVERGE` with a hint to `navigation.geodesic.inverse`

#### Scenario: Vincenty comparison
- **WHEN** the Vincenty inverse converges for JFK to LHR
- **THEN** the result includes the difference from the Karney distance (sub-millimeter)

### Requirement: Spherical great-circle methods
Spherical tools (haversine distance, spherical direct and inverse, spherical midpoint, intermediate point) SHALL require or default the sphere radius (default mean radius R1 = 6,371,008.771 m) and SHALL report the difference from the ellipsoidal result for the same inputs, so users see the approximation error.

#### Scenario: Haversine error shown
- **WHEN** haversine distance is computed for JFK to LHR with R1
- **THEN** the distance ≈ 5,540,019 m, and the result reports that it is ≈ 14,890 m (0.27%) shorter than the ellipsoidal geodesic

### Requirement: Rhumb lines on the ellipsoid
Rhumb-line (loxodrome) direct and inverse SHALL be computed on the ellipsoid (Karney's Rhumb algorithm), returning constant course and distance, and SHALL handle meridional, equatorial, and pole-reaching rhumbs. The inverse SHALL also report the extra distance compared with the geodesic.

#### Scenario: Rhumb vs geodesic
- **WHEN** the rhumb inverse is computed for JFK to LHR
- **THEN** the constant course ≈ 77.968° (±0.001°), distance ≈ 5,774,190 m (3,117.8 NM, ±1 m), and the result reports ≈ 219.3 km (3.9%) longer than the geodesic

#### Scenario: Rhumb to the pole
- **WHEN** the rhumb direct problem would pass a pole
- **THEN** the result stops at the pole with warning `RHUMB_REACHES_POLE` and the remaining distance

### Requirement: Waypoints, densification, and intermediate points
Tools SHALL generate points along a geodesic or rhumb line at a fixed spacing, at N equal intervals, or at given fractions, and SHALL compute the midpoint and any intermediate point. The output SHALL include cumulative distance and azimuth at each point, and be exportable as GPX route or GeoJSON.

#### Scenario: Equal intervals
- **WHEN** 10 equal intervals are requested for JFK to LHR
- **THEN** 11 points are returned (including both endpoints), each ≈ 555,490.879 m apart along the geodesic

### Requirement: Geodesic intersections and vertex
Tools SHALL compute the intersection point(s) of two geodesics (given by point and azimuth, or by segment endpoints), using Karney's intersection algorithm. They SHALL report when segments do not intersect within their extents, and report the closest intersection when lines intersect multiple times. A tool SHALL compute a geodesic's vertex (maximum latitude and its longitude).

#### Scenario: Non-intersecting segments
- **WHEN** two segments whose full geodesics intersect outside both segments are tested
- **THEN** the result reports the intersection of the extended geodesics and flags that it lies outside both segments

### Requirement: Ellipsoid choice and custom ellipsoids
All geodesic tools SHALL accept any catalog ellipsoid or a custom (a, f) and SHALL use the exact method for |f| > 0.02 (per GeographicLib's GeodesicExact guidance).

#### Scenario: Mars ellipsoid
- **WHEN** a user supplies a = 3,396,190 m and f = 1/169.894 (Mars)
- **THEN** the inverse runs with the series method and `meta.model` names the ellipsoid parameters

#### Scenario: High flattening uses the exact method
- **WHEN** a user supplies a = 6,378,137 m and f = 1/10
- **THEN** `meta.model` names GeodesicExact, and the distance agrees with GeographicLib `GeodSolve -E` within 1e-9 relative

### Requirement: Visualization
Geodesic tools SHALL render the geodesic (solid), the rhumb line (dashed) and, on request, the spherical great circle for comparison. They SHALL label distance, initial and final courses, and the vertex, on both the 3D globe and the 2D map.

#### Scenario: Comparison overlay
- **WHEN** a user enables "compare methods"
- **THEN** the canvas shows all three paths with a legend and their distances
