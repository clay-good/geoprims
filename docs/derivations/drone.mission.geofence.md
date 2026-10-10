<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Geofence (`drone.mission.geofence`)

## Method

A geofence here is every place within a set distance of the planned area: the area itself, a route, or a single point, grown outward by that distance. The tool builds the fence with the geodesic buffer the geometry tools use, and reports the ground it encloses and its perimeter. Given waypoints, it measures each one's geodesic distance to the area and flags those past the fence, with how far past, and, when a closer warning distance is set, those past that line but still inside the fence.

## Equations

- Fence: the round geodesic buffer of the area at the fence distance (`geometry.buffer.geodesic`).
- For a waypoint at geodesic distance x from the area (0 inside it): outside when x > d, by x − d; in the warning band when w < x ≤ d, by x − w.
- Outside count: the waypoints with x > d. Warning count: those in the band.
- For a convex area on a plane the fence encloses A + P·d + π·d² (Steiner), which the first vectors use on the equator.

## Symbols and units

d the fence distance and w the warning distance, in meters by default; A and P the area's area and perimeter. Vertices and waypoints are latitude and longitude on WGS 84. The area enclosed is in square kilometers.

## Domain

One vertex is a point, two a line, three or more a polygon, unless the shape is named. The warning distance must be less than the fence distance. An outline that repeats itself is refused, as is a fence too large for the buffer.

## Approximations

Distances are geodesic and the area's edges are geodesics between its corners. The fence outline is a polygon standing in for a curve, so its enclosed area is a little under the exact one (within 0.2% in the vectors) and the tool reports its largest departure. GPS error and wind drift are not part of the fence: the distance has to leave room for them.

## Worked example

- sourcePublisher: GEOS (through Shapely) and PROJ (through pyproj)
- sourceTitle: Shapely 2.0.7 with GEOS 3.11.4, on an azimuthal equidistant plane from PROJ 9.3.0
- sourceEdition: run 2026-10-10
- sourceLocator: tools/vectors/gen_geofence.py: ten irregular and concave fields at latitudes from 55° S to 69° N, each projected at its own centroid, with seven waypoints apiece measured to the field by GEOS
- independent: yes
- inputs: for example a five-sided field near 40.44° N, 80° W with a 50 m fence, and a notched field near London with a 30 m fence and a 20 m warning line
- outputs: which waypoints are flagged, in order, and how far each is past its line, as GEOS measures it on the plane
- tolerance: 2 mm in each distance; exact in the counts and the order
- verifiedBy: golden vectors v012 through v021, run by the core on every build
- verifiedOn: 2026-10-10

An azimuthal equidistant plane keeps distances within a kilometer of its center true to parts per billion, so GEOS on that plane is a fair second opinion on a geodesic distance at this size. Waypoints within 5 cm of a line are left out of the vectors, so none turns on the last millimeter.

## Differential tests

- `tools/vectors/gen_geofence.py`: GEOS and PROJ, as above; it needs Shapely and pyproj and replaces only its own rows
- `tools/vectors/gen_drone.py`: rectangles with an edge on the equator and one on the prime meridian, where a waypoint's distance is known in closed form, with Steiner's formula for the enclosed area; a route and a point; and the refused inputs
- `core/vectors/drone.mission.geofence.jsonl`: those vectors, run through the core on every build; v008 is superseded, as it pinned a warning's place in the list

## Invariants

- `core/crates/gp-drone/tests/mission.rs` `geofence_invariants`: at four latitudes, waypoints placed known geodesic distances out from a field's edge are flagged in order by exactly the excess, one inside is not, and a wider fence encloses more and flags fewer
- `core/crates/gp-drone/tests/mission.rs` `waypoint_outside_the_fence_is_flagged_with_index_and_distance`: the spec scenario, a survey grid's own waypoints inside the fence and one 62 m out flagged as 12 m beyond a 50 m fence, with the warning code
