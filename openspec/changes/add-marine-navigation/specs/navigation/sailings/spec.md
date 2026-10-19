## Purpose

Solves the sailings as Bowditch states them (plane, traverse, parallel, mid-latitude, Mercator, great-circle with composite option), in the forms a navigator or examinee checks, with the exact ellipsoidal answer shown beside each.

## ADDED Requirements

### Requirement: Plane, parallel, and traverse sailing
The plane-sailing tool SHALL solve either problem: from course and distance, the difference of latitude l and departure p (l = D cos C, p = D sin C); or from l and p, the course and distance. The parallel-sailing tool SHALL convert between departure and difference of longitude on a parallel (p = DLo cos L). The traverse tool SHALL sum several courses and distances to one l and p and the resulting course and distance made good. Distances SHALL be in nautical miles, angles in degrees to 0.1° for display and full precision in the machine value.

#### Scenario: Plane sailing from course and distance
- **WHEN** a ship steams 188.0 nm on course 005°
- **THEN** l = 187.3' N and p = 16.4 nm E, matching Bowditch §1215

#### Scenario: Parallel sailing
- **WHEN** DLo = 3°30' on latitude 49°30'
- **THEN** the departure is 136.4 nm (±0.05 nm), matching Bowditch §1217

### Requirement: Mid-latitude and Mercator sailing
The mid-latitude tool SHALL solve course and distance between two positions, and the position reached from a departure, course, and distance, using the mean latitude (with Bowditch's note that the "middle latitude" correction is not applied). The Mercator tool SHALL do the same with meridional parts computed on the WGS 84 spheroid (as Bowditch Table 6), reporting M₁, M₂, m, and DLo. Both SHALL also report the exact ellipsoidal rhumb-line result from `navigation.rhumb` and the difference in course (°) and distance (nm).

#### Scenario: Mid-latitude position reached
- **WHEN** a ship sails 1,253 nm on course 070° from 15°17.0'N 151°37.0'E
- **THEN** the arrival is 22°25.6'N 172°21.2'E within 0.1', matching Bowditch §1218

#### Scenario: Mercator course and distance
- **WHEN** the Mercator sailing is solved from 32°14.7'N 66°28.9'W to 36°58.7'N 75°42.2'W
- **THEN** m = 343.7, DLo = 553.3', course 301.8° T, and distance 538.9 nm, each within Bowditch's printed precision (§1219)

#### Scenario: Rhumb line beside
- **WHEN** any Mercator sailing is solved
- **THEN** the result shows the exact ellipsoidal rhumb course and distance and their differences

### Requirement: Great-circle and composite sailing
The great-circle tool SHALL compute the spherical initial course and distance (Bowditch §1208), the vertex (§1211), and points along the track at a chosen DLo interval (§1209), and SHALL report the Karney geodesic distance and course with the difference. With a limiting latitude it SHALL compute the composite track (§1213): the two great-circle legs tangent to the limiting parallel and the parallel leg, with each leg's course and distance and the total. The track SHALL draw on the map with the vertex and limiting parallel marked.

#### Scenario: Great circle across the Indian Ocean
- **WHEN** the track from 22°S 116°E to 20°S 031°E is computed
- **THEN** the distance is 4,693.8 nm and the initial course 253.0° T, within Bowditch's printed precision (§1208), and the geodesic difference is shown

#### Scenario: Composite below a limiting latitude
- **WHEN** a track whose vertex lies at 51°N is limited to 45°N
- **THEN** the result has three legs, the middle leg runs along 45°N, and the total exceeds the unlimited great-circle distance

#### Scenario: Antipodal points
- **WHEN** the two positions are antipodal
- **THEN** the result is `DEGENERATE_GEOMETRY` stating that every great circle through them has the same length
