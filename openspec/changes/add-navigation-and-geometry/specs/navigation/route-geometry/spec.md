## Purpose

Provides route-level navigation math (deviation from course, turn anticipation, intercepts, multi-leg totals, and time-speed-distance) that pilots, mariners, drone operators, and UAM engineers build flight and mission plans from.

## ADDED Requirements

### Requirement: Cross-track and along-track distance
Given a path (geodesic segment from A to B, or a rhumb segment) and a point P, the tool SHALL compute the cross-track distance (signed: positive right of course when facing from A to B), the along-track distance from A to the foot of the perpendicular, the foot point itself, and whether the foot lies within the segment. On the ellipsoid, the foot SHALL be the point on the geodesic closest to P, found to 1 mm or better, and the result SHALL agree with a spherical formula only when the spherical method is explicitly selected.

#### Scenario: Point right of course
- **WHEN** P lies south-east of the geodesic from JFK to LHR near the start
- **THEN** the cross-track distance is positive (right of course) and the foot point lies within the segment

#### Scenario: Foot beyond the end
- **WHEN** P lies beyond B along the course
- **THEN** the along-track distance exceeds the segment length and the result flags `FOOT_OUTSIDE_SEGMENT` and also returns the distance from P to B

### Requirement: Closest point on a polyline route
Given a multi-leg route and a point, a tool SHALL return the closest point on the route, the leg index, along-route distance, and cross-track distance relative to that leg.

#### Scenario: Leg identification
- **WHEN** P is closest to the third leg of a five-leg route
- **THEN** the result reports leg index 3 (1-based) and the along-route distance including the first two legs

### Requirement: Course intersections and intercepts
Tools SHALL compute where two courses (each a point plus course, geodesic or rhumb) intersect, and SHALL solve the intercept problem: the course to steer from a current position at a given speed to intercept a moving target (position, course, speed), returning intercept point and time, or `NO_SOLUTION` when the target cannot be reached.

#### Scenario: Unreachable target
- **WHEN** the pursuer's speed is less than the target's and the target is moving directly away
- **THEN** the result is an error with code `NO_SOLUTION` and a message explaining that the target is opening faster than the pursuer can close

### Requirement: Fly-by turn anticipation and turn geometry
Given the inbound course, outbound course, groundspeed or true airspeed, and bank angle (or turn rate), a tool SHALL compute turn radius R = V² / (g · tan φ), the turn anticipation distance (lead distance) R · tan(Δψ/2) for a fly-by waypoint, the arc length, the time in turn, and the turn start and end points on the route. It SHALL flag course changes greater than 120° as unsuitable for fly-by turns (`FLY_OVER_RECOMMENDED`).

#### Scenario: 90° fly-by turn
- **WHEN** V = 120 kt, bank = 25°, and the course changes by 90°
- **THEN** turn radius ≈ 833.4 m (0.450 NM) and lead distance ≈ 833.4 m (each ±0.1 m)

#### Scenario: Standard-rate radius
- **WHEN** a standard-rate turn (3°/s) is requested at 120 kt
- **THEN** the bank angle ≈ 18.24° (±0.01°) and turn radius ≈ 1,179.0 m (±0.1 m)

### Requirement: Range rings and circles
A tool SHALL generate a geodesic circle (all points at distance r from a center), densified to the canvas accuracy rule, correctly handling circles that cross the antimeridian or enclose a pole, and SHALL output multiple rings at given radii.

#### Scenario: Circle enclosing the pole
- **WHEN** a 1,500 km ring is centered at 85° N
- **THEN** the returned polygon encloses the North Pole and is valid GeoJSON (per RFC 7946 antimeridian cutting)

### Requirement: Multi-leg route totals
A route tool SHALL accept an ordered list of waypoints (any coordinate notation) and a path type per leg (geodesic or rhumb), and return per-leg distance, initial and final true course, magnetic course (via geomagnetism for a given date), cumulative distance, and total distance. With a groundspeed, it SHALL also return leg times and ETAs from a departure time.

#### Scenario: Route with magnetic courses
- **WHEN** a 4-waypoint route is computed with date 2026-09-18
- **THEN** each leg shows true and magnetic courses, with the declination and model used per leg

### Requirement: Time-speed-distance
Tools SHALL solve any one of time, speed, or distance from the other two with unit handling, plus ETA and ETE from departure time and time zone offset (explicit UTC offset; no time-zone database lookup).

#### Scenario: Solve for time
- **WHEN** distance = 250 NM and groundspeed = 125 kt
- **THEN** time = 2 h 00 min

### Requirement: Closest point of approach between moving objects
Given two objects with positions, courses, and speeds (in a local tangent plane, valid for separations under 500 km), a tool SHALL compute the time of closest point of approach (CPA), the separation at CPA, and the bearing and range at CPA. If CPA is in the past, it SHALL return the current separation with warning `DIVERGING` (not an error).

#### Scenario: CPA in local plane
- **WHEN** object A at the origin moves east at 10 m/s and object B at (1,000 m E, 1,200 m N) moves south at 10 m/s
- **THEN** CPA occurs at t = 110 s with separation ≈ 141.42 m

### Requirement: Route visualization
Route tools SHALL draw legs, waypoints with labels, turn arcs for fly-by turns, cross-track offsets as perpendicular markers, and range rings.

#### Scenario: Turn arcs drawn
- **WHEN** a route with fly-by turns is computed with a bank angle
- **THEN** each turn is drawn as an arc tangent to both legs
