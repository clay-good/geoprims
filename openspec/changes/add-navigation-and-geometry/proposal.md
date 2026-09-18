## Why

"Distance between two points" is the most searched spatial calculation, and most online answers use a spherical haversine that is off by up to about 0.5%. That is 14.9 km on a transatlantic leg. They ignore rhumb vs great-circle differences, and Vincenty's inverse fails near antipodes.

Pilots, mariners, drone operators, and developers also need the route-level math built on top: cross-track error, turn anticipation, intercepts, closest approach, line of sight over a curved Earth, and the computational geometry (areas, buffers, hulls, simplification) that spatial software is made of.

Depends on: `establish-platform-foundation`, `add-geodesy-suite` (parsing and frames).

## What Changes

Adds the `navigation` domain (about 46 operations and 58 endpoints) and the `geometry` domain (about 38 operations and 44 endpoints).

- **Geodesics:** Karney direct and inverse (default), Vincenty direct and inverse (labeled, with convergence failure reporting), spherical great circle with a stated radius, rhumb lines on the ellipsoid, waypoints and densification, intersections, and vertex (max latitude).
- **Route geometry:** cross-track and along-track distance, closest point, course intersections, fly-by turn anticipation, range rings, multi-leg routes with totals, and time-speed-distance.
- **Line of sight:** geometric, optical, and radio horizons; mutual visibility of two elevated points; hidden height; dip of the horizon; slant and ground range.
- **3D vectors:** 3D distance with altitude, azimuth/elevation/slant range, vector algebra, and closest point of approach between moving objects.
- **Computational geometry:** geodesic and planar area, perimeter, centroid, convex hull, bounding rectangles, minimum enclosing circle, geodesic buffers, simplification (Ramer-Douglas-Peucker, Visvalingam-Whyatt), point-in-polygon, validity and repair, boolean operations, densification, triangulation, Voronoi, and shape distances.

## Capabilities

### New Capabilities

- `navigation/geodesic`: Ellipsoidal and spherical geodesic and rhumb-line problems.
- `navigation/route-geometry`: Route-level navigation math built on geodesics.
- `navigation/line-of-sight`: Horizon and visibility over a curved, refracting Earth.
- `navigation/vector-3d`: 3D positional vectors, slant geometry, and relative motion.
- `geometry/computational`: Planar and ellipsoidal computational geometry.

### Modified Capabilities

None.

## Non-goals

- Airway or route databases, navaids, or airport data (no operational data, per foundation).
- Terrain-aware line of sight. It lives in `add-spatial-indexing-and-raster` (raster/terrain).
- Weapon or ballistic trajectory calculations (excluded by foundation D12).

## Impact

- `core/gp-navigation` and `core/gp-geometry` crates.
- Differential references: GeographicLib C++ (`GeodSolve`, `RhumbSolve`, `Planimeter`, `IntersectTool`), GEOS (via a CI container) for planar geometry, and Movable Type formulas as the spherical cross-check.
