## Purpose

Provides robust computational geometry on both the plane and the ellipsoid: measurement, envelopes, buffers, simplification, predicates, validity, and set operations. These let users measure and shape boundaries, geofences, and survey parcels correctly.

## ADDED Requirements

### Requirement: Geodesic area and perimeter
The area tool SHALL compute the area and perimeter of a polygon (with holes) on the ellipsoid, using edges that are geodesics (default) or rhumb lines. Its accuracy SHALL match GeographicLib `Planimeter` within 1e-6 relative. It SHALL handle polygons that cross the antimeridian or enclose a pole, and SHALL report the ring orientation. A planar-area mode (shoelace formula) SHALL be available for projected coordinates, and when used on geographic coordinates it SHALL warn `PLANAR_ON_GEOGRAPHIC`.

#### Scenario: Colorado-shaped rectangle
- **WHEN** the polygon (37° N, 109.05° W) → (41° N, 109.05° W) → (41° N, 102.05° W) → (37° N, 102.05° W) is measured on WGS 84 with geodesic edges
- **THEN** area ≈ 269,154.55 km² and perimeter ≈ 2,099.854 km, and the result reports clockwise orientation

#### Scenario: Polygon around the pole
- **WHEN** a ring at 80° N circling the pole is measured
- **THEN** the area is that of the polar cap north of the ring (not the rest of the Earth), and the pole is reported as enclosed

#### Scenario: Shoelace on degrees warns
- **WHEN** planar mode is applied to latitude/longitude input
- **THEN** the result includes `PLANAR_ON_GEOGRAPHIC` and suggests the geodesic mode

### Requirement: Centroids and representative points
Tools SHALL compute a polygon's geodesic centroid (area-weighted on the ellipsoid via a local equal-area projection, stated), planar centroid for projected input, and a guaranteed-interior representative point (pole of inaccessibility within a tolerance).

#### Scenario: Centroid outside a C-shape
- **WHEN** the centroid of a C-shaped polygon falls outside the polygon
- **THEN** the result flags `CENTROID_OUTSIDE` and also returns the interior representative point

### Requirement: Envelopes
Tools SHALL compute the convex hull (planar and spherical), the axis-aligned bounding box (with antimeridian-aware longitude ranges, west > east when crossing), the minimum-area rotated bounding rectangle (MBR), and the minimum enclosing circle (geodesic).

#### Scenario: Antimeridian bbox
- **WHEN** the bbox of points at longitudes 170° E and 170° W is computed
- **THEN** the result is west = 170, east = -170 (a 20° span), flagged `CROSSES_ANTIMERIDIAN`, not a 340° span

### Requirement: Geodesic buffers
The buffer tool SHALL buffer points, lines, and polygons by a distance on the ellipsoid (positive or negative) with selectable join style (round, mitre, bevel) and cap style (round, flat, square). The offset boundary SHALL be within 0.1% of the requested distance, or 0.5 m, whichever is larger, at every vertex. Buffers across the antimeridian and around poles SHALL produce valid geometry.

#### Scenario: Geofence buffer
- **WHEN** a polygon is buffered outward by 500 m
- **THEN** every output vertex is 500 m ± 0.5 m from the input boundary (geodesic distance), and the output is valid

#### Scenario: Negative buffer collapses
- **WHEN** a 100 m wide polygon is buffered by -60 m
- **THEN** the result is an empty geometry with warning `BUFFER_COLLAPSED`

### Requirement: Simplification
Tools SHALL simplify polylines and polygons by Ramer-Douglas-Peucker (tolerance as a distance in meters, applied geodesically or in a stated projection) and Visvalingam-Whyatt (area threshold or target vertex count), with a topology-preserving option that never introduces self-intersections, and SHALL report vertices removed and the maximum deviation.

#### Scenario: Topology preserved
- **WHEN** a polygon with a narrow inlet is simplified with topology preservation on
- **THEN** the output has no self-intersections, and the maximum deviation is reported

### Requirement: Predicates
Tools SHALL test point-in-polygon (with holes) by both winding-number and even-odd rules, reporting `inside`, `outside`, or `on-boundary` (within a stated tolerance), and SHALL test intersects, contains, within, touches, crosses, overlaps, and disjoint for pairs of geometries using robust (exact) orientation predicates. For geographic input, edges SHALL be treated as geodesics unless the user selects planar.

#### Scenario: Point on boundary
- **WHEN** a point lies exactly on a polygon edge
- **THEN** the result is `on-boundary`

#### Scenario: Winding vs even-odd differ
- **WHEN** a self-overlapping polygon is tested at a doubly wound point
- **THEN** winding number reports inside (winding 2) and even-odd reports outside, and both are returned

### Requirement: Validity and repair
A tool SHALL validate geometry per OGC Simple Features (ring closure, self-intersection, hole containment, duplicate vertices, spikes, and orientation per RFC 7946 for GeoJSON output), reporting each problem with its location. It SHALL offer repair (make-valid) that preserves area where possible.

#### Scenario: Bow-tie repaired
- **WHEN** a self-intersecting bow-tie polygon is repaired
- **THEN** the output is a valid MultiPolygon of two triangles, and the report locates the original crossing

### Requirement: Boolean operations
Tools SHALL compute union, intersection, difference, and symmetric difference of polygons (planar in a stated projection, or geodesic-edge via densification with stated tolerance), returning valid geometry.

#### Scenario: Geofence overlap
- **WHEN** the intersection of two overlapping geofences is computed
- **THEN** the result is the overlap polygon with its geodesic area

### Requirement: Densification, triangulation, Voronoi, and distances
Tools SHALL densify lines to a maximum segment length, compute Delaunay triangulation and Voronoi diagrams (planar, and spherical for global point sets), and compute distances between geometries (minimum distance, Hausdorff, discrete Fréchet).

#### Scenario: Track similarity
- **WHEN** two GPS tracks are compared with discrete Fréchet distance
- **THEN** the result gives the distance in meters and the index pair where it occurs

### Requirement: Input size limits
Geometry tools SHALL accept up to 1,000,000 vertices per request in the web app (500,000 via MCP by default) and return `LIMIT_EXCEEDED` above that.

#### Scenario: Oversized input
- **WHEN** a 2,000,000-vertex polygon is submitted
- **THEN** the tool returns `LIMIT_EXCEEDED` before processing
