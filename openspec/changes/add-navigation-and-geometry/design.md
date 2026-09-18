## Context

Motivation is in `proposal.md`. This change reuses geodesy parsing, frames, and magnetic tools (`add-geodesy-suite`). Reference facts (from `docs/research/01`):

- Karney's geodesic algorithms converge everywhere with ~15 nm round-off. Vincenty (~0.5 mm) fails near antipodes.
- GeographicLib's Rhumb and Intersect classes exist in C++ but not in `geographiclib-rs`.
- Rule-of-thumb constants for horizons disagree (1.06 / 1.14 / 1.17 / 1.23 × √h NM) because they assume different refraction.

## Goals / Non-Goals

**Goals:**
- Ellipsoidal answers by default. Every approximation reports its error against the ellipsoidal answer.
- Robust planar predicates (exact orientation tests) so point-in-polygon and boolean operations never flip on floating-point noise.

**Non-Goals:**
- Terrain (see raster). Navigation databases. Great-circle routing around obstacles or airspace.

## Decisions

### N1. Karney default; approximations as comparison tools
The inverse, direct, and rhumb tools use GeographicLib algorithms, with Rhumb and Intersect ported from C++. Vincenty and haversine are separate endpoints because users search for them by name. Each also returns the delta against Karney. This meets users where they are and teaches the difference.

### N2. Cross-track on the ellipsoid by closest-point iteration
Spherical cross-track formulas are wrong by up to ~0.5% on the ellipsoid. The foot point is found by Karney's method: the point on the geodesic where the geodesic from P meets it at 90°. It is solved with the intersection machinery (Newton iteration on along-track distance, converging to 1 mm). The spherical formula remains a selectable mode.

### N3. Local tangent plane for relative motion
CPA and intercept problems run in a local ENU plane at the midpoint, valid under 500 km separation (declared limit). They are closed-form and fast. Larger separations return `LIMIT_EXCEEDED` with an explanation.

### N4. Geometry engine
- `geo` crate algorithms (convex hull, simplification, Visvalingam-Whyatt, Fréchet, Hausdorff) on robust predicates (`robust` crate, Shewchuk).
- Boolean operations and offsets use `i_overlay`, with `clipper2` as a fallback candidate if `i_overlay` fails the GEOS differential suite.
- Geodesic buffering: build the offset in an azimuthal equidistant projection centered on the geometry's centroid for small extents (< 1,000 km). Split larger geometries into tiles. Validate the result by sampling geodesic distances (the tolerance from the spec). A pure-ellipsoidal offset algorithm was rejected as research-grade and slow; the projection approach is verified by measurement instead.
- Geodesic-edge boolean operations densify edges to the canvas tolerance, then run planar operations in a suitable local projection, and report the tolerance.

### N5. Antimeridian and pole policy
Internally, rings are unwrapped longitude-continuous. Output GeoJSON is cut at ±180° per RFC 7946 §3.1.9. Pole-enclosing rings are closed via the pole for area and via RFC 7946 conventions for output.

## Tool inventory (targets)

| Domain | Group | Operations | Examples |
|---|---|---|---|
| navigation | `geodesic` | 16 | inverse, direct, vincenty-inverse/direct, haversine, spherical-direct/inverse, spherical-midpoint, intermediate-point, midpoint, rhumb-inverse/direct, waypoints, intersection, vertex, line-bbox |
| navigation | `route` | 12 | cross-track, closest-point-on-route, course-intersection, intercept, fly-by-turn, standard-rate-turn, range-rings, route-totals, parallel-offset, tsd-solve, eta, cpa-2d |
| navigation | `los` | 6 | horizon, mutual-visibility, hidden-height, dip, geographic-range, fresnel-clearance |
| navigation | `vector` | 12 | distance-3d, look-angles, add, subtract, scale, dot, cross, angle-between, projection, polar-cartesian, relative-velocity, cpa-3d |
| **navigation** | | **46 ops / 58 endpoints** | +12 generated endpoints: distance between two points given in MGRS, DMS, UTM, Maidenhead, and so on (`mgrs-distance`, `dms-distance`, …) |
| geometry | `measure` | 7 | area-geodesic, area-planar, perimeter, line-length, centroid, representative-point, orientation |
| geometry | `envelope` | 6 | convex-hull-planar, convex-hull-spherical, bbox, mbr, min-enclosing-circle, densify |
| geometry | `buffer` | 3 | buffer-point, buffer-line, buffer-polygon |
| geometry | `simplify` | 2 | rdp, visvalingam |
| geometry | `predicate` | 8 | point-in-polygon, intersects, contains, within, touches, crosses, overlaps, disjoint |
| geometry | `validity` | 2 | validate, make-valid |
| geometry | `boolean` | 4 | union, intersection, difference, sym-difference |
| geometry | `mesh` | 3 | delaunay, voronoi-planar, voronoi-spherical |
| geometry | `distance` | 3 | min-distance, hausdorff, frechet |
| **geometry** | | **38 ops / 44 endpoints** | +6 generated endpoints: file-oriented variants such as `geojson-area`, `kml-area`, `gpx-length` |

## Risks / Trade-offs

- **[Buffer accuracy near poles and for huge extents]** → Tiling plus measured validation. `LIMIT_EXCEEDED` above a declared extent (5,000 km).
- **[i_overlay robustness on degenerate inputs]** → GEOS differential tests on a corpus of known-hard cases (the JTS test suite geometries).
- **[Users expect haversine numbers from other sites]** → The haversine endpoint gives exactly that, and shows the ellipsoidal truth beside it.

## Migration Plan

Not applicable.

## Open Questions

None that affect the specs.
