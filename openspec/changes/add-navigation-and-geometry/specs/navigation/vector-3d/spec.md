## Purpose

Provides true 3D positional math, including altitude-aware distance, slant geometry, and vector algebra, for drones, aircraft, antennas, and anything else that is not on the ground.

## ADDED Requirements

### Requirement: 3D distance between geodetic points
Given two points with latitude, longitude, and height (with the height reference declared: ellipsoidal, or orthometric with a geoid model), the tool SHALL compute the straight-line (chord) 3D distance via ECEF, the surface geodesic distance, the height difference, and the elevation angle from the first point to the second. Orthometric heights SHALL be converted to ellipsoidal first, and the result SHALL state which geoid was used.

#### Scenario: Drone to ground station
- **WHEN** the distance is computed between a ground station at (40° N, 105° W, 250 m HAE) and a drone 2,000 m due north along the geodesic at 370 m HAE
- **THEN** the result gives slant range ≈ 2,003.694 m, ground distance 2,000 m, height difference 120 m, and elevation angle ≈ 3.4245°, and notes that the flat-Earth approximation (2,003.597 m, 3.4336°) ignores curvature

#### Scenario: Mixed height references rejected
- **WHEN** one point's height is HAE and the other's is MSL with no geoid chosen
- **THEN** the tool returns `INVALID_INPUT` asking for a geoid model to reconcile height references

### Requirement: Azimuth, elevation, slant range (look angles)
A tool SHALL compute azimuth, elevation, and slant range from an observer to a target (both geodetic), including targets below the horizon (negative elevation), and SHALL optionally apply standard atmospheric refraction to elevation.

#### Scenario: Target below horizon
- **WHEN** a target is beyond the geometric horizon
- **THEN** elevation is negative and the result includes `BELOW_HORIZON`

### Requirement: Vector algebra
Tools SHALL provide 2D and 3D vector operations with units: addition, subtraction, scaling, dot product, cross product, magnitude, unit vector, angle between vectors, projection, and conversion between polar/spherical (magnitude, direction, elevation) and Cartesian forms. Direction conventions (mathematical counterclockwise from +x vs navigational clockwise from north) SHALL be selectable and stated.

#### Scenario: Navigational convention
- **WHEN** a vector of magnitude 10 at direction 090° (navigational) is converted to Cartesian (east, north)
- **THEN** the result is (10, 0)

### Requirement: Relative motion in 3D
A tool SHALL compute 3D closest point of approach for two objects with positions and velocity vectors (local ENU), returning time, 3D separation, horizontal and vertical separation at CPA.

#### Scenario: Vertical separation at CPA
- **WHEN** two aircraft converge head-on with 1,000 ft vertical separation
- **THEN** CPA reports horizontal separation near 0 and vertical separation 1,000 ft

### Requirement: Vector diagrams
Vector tools SHALL render vectors head-to-tail in the vector-diagram canvas mode, with magnitudes and angles labeled, and 3D results in a rotatable ENU view.

#### Scenario: Head-to-tail sum
- **WHEN** three vectors are added
- **THEN** the diagram shows them head-to-tail with the resultant highlighted
