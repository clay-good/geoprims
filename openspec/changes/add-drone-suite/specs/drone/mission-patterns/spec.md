## Purpose

Generates vendor-neutral flight patterns (survey grids, corridors, orbits, facade scans) and geofences from user geometry and camera settings, as waypoint lists drawn on the map and exportable to standard formats.

## ADDED Requirements

### Requirement: Survey grid over a polygon
Given an area polygon, line spacing (or camera and overlap settings from photogrammetry), flight direction (degrees true, or "auto" for the fewest lines), overshoot distance, and height above ground, the tool SHALL generate a serpentine (lawnmower) path clipped to the polygon (including polygons with holes). It SHALL return waypoints, line count, total path length, turn count, and estimated flight time at a groundspeed. Lines and photos SHALL follow the image-count requirement's published method (⌈width / spacing⌉ + 1 lines; ⌈length / photo spacing⌉ + 1 photos per line plus the extra photos past each end), and each line SHALL be flown far enough past its ends to take those photos. An optional crosshatch SHALL add a second perpendicular grid.

#### Scenario: Auto direction minimizes lines
- **WHEN** direction is "auto" for a long, thin rectangle
- **THEN** the lines run parallel to the rectangle's long axis and the line count equals ⌈width / spacing⌉ + 1, the first and last lines on or just past the long edges (the published flight-line count, Penn State GEOG 892, "Designing a Flight Route")

#### Scenario: Polygon with a hole
- **WHEN** the polygon has a hole (a no-fly area)
- **THEN** no path segment crosses the hole, and transit segments route around it along the hole boundary with a user-set buffer

### Requirement: Corridor mapping
Given a centerline polyline and a corridor width, the tool SHALL generate a set of parallel lines offset from the centerline (number from width and spacing) that follow bends, with turn handling at the ends.

#### Scenario: Pipeline corridor
- **WHEN** a 5 km polyline with 120 m width and 52.5 m spacing is used
- **THEN** three offset lines are generated (at -52.5, 0, +52.5 m), covering the corridor

### Requirement: Orbit and point of interest
Given a center, radius, height, direction (clockwise or counterclockwise), and number of photos or angular step, the tool SHALL generate an orbit (geodesic circle) with a camera heading and gimbal pitch per waypoint that keeps a target at a given height centered.

#### Scenario: Gimbal pitch for a tower top
- **WHEN** orbiting a 60 m tower at radius 50 m and height 80 m, targeting the tower top
- **THEN** each waypoint's gimbal pitch ≈ -21.8° (atan(20/50)) and the heading points at the center

### Requirement: Facade and structure scan
Given a facade line (two points), a standoff distance, a height range, and vertical and horizontal overlaps (with the camera), the tool SHALL generate a vertical lawnmower pattern parallel to the facade with a fixed standoff and a horizontal camera.

#### Scenario: Facade GSD
- **WHEN** a 30 m standoff is used
- **THEN** the facade GSD is computed with the standoff as the object distance and reported

### Requirement: Geofence generation
Given a geometry (point, line, polygon) and a buffer distance, the tool SHALL generate a geodesic geofence polygon (via geometry buffers). It SHALL generate an optional inner "warning" fence at a second distance, report the geofence area and perimeter, and flag any mission waypoint outside the fence.

#### Scenario: Waypoint outside fence
- **WHEN** a generated survey grid has a waypoint 12 m outside a geofence
- **THEN** the result flags `WAYPOINT_OUTSIDE_GEOFENCE` with the waypoint index and distance

### Requirement: Altitude references on every waypoint
Every generated waypoint SHALL carry height with an explicit reference: AGL (constant or terrain-following using the terrain asset), relative-to-takeoff, MSL, or HAE. Conversions SHALL use the geodesy height tools. Terrain-following heights SHALL state the DEM and its accuracy.

#### Scenario: Terrain-following requires acknowledgment
- **WHEN** a user enables terrain-following export over the GLO-30 surface model
- **THEN** export is blocked until the user acknowledges that the model includes trees and buildings inconsistently and sets a clearance margin (default 15 m) that is added to every waypoint

#### Scenario: Terrain-following
- **WHEN** terrain-following at 80 m AGL is enabled over hilly terrain and the user acknowledges the surface-model notice
- **THEN** each waypoint's MSL height is computed from the DEM plus 80 m, and the DEM name and vertical accuracy are listed

### Requirement: Export
Patterns SHALL export as KML (with altitude mode matching the height reference), GeoJSON (with per-waypoint properties), and CSV (lat, lon, height, reference, heading, gimbal pitch, action). The export SHALL include a header comment with tool version and a not-for-navigation notice.

#### Scenario: KML altitude mode
- **WHEN** a pattern with AGL heights is exported to KML
- **THEN** the KML uses `relativeToGround` altitude mode

### Requirement: Pattern visualization
Patterns SHALL be drawn on the map and 3D globe with direction arrows, turn points, photo trigger points (optional), the camera footprint coverage (optional), and the geofence.

#### Scenario: Coverage overlay
- **WHEN** the coverage overlay is enabled
- **THEN** image footprints are drawn with overlap shading
