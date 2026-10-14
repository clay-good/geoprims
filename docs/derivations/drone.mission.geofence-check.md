<!-- Derivation note (trust/correctness-program layer A). -->
# Check waypoints against a geofence (`drone.mission.geofence-check`)

## Method

A geofence a fixed distance around an area is the set of points within that geodesic distance of it. A waypoint is outside the fence when its distance to the area is more than the fence distance, and past a warning fence when it is more than the warning distance. The operation measures each waypoint's geodesic distance to the area and sorts it. It shares one calculation path with `drone.mission.geofence`, which also returns the fence's outline.

## Equations

- d_i = geodesic distance from waypoint i to the nearest point of the area (0 inside it)
- Outside the geofence when d_i > D, by d_i − D
- Past the warning fence when W < d_i ≤ D, by d_i − W

## Symbols and units

D fence distance and W warning distance, in meters or feet, with 0 < W < D. Waypoints and area corners in latitude and longitude on WGS 84.

## Domain

An area of three or more distinct corners (or a line or a point, by the shape setting), a positive fence distance, and at least one waypoint.

## Approximations

Distances are geodesic on the WGS 84 ellipsoid. Only waypoints are tested, not the legs between them.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: add-drone-suite specification
- sourceEdition: 2026
- sourceLocator: drone/mission-patterns, requirement "Geofence" (waypoints flagged with their distance beyond the fence); checked by meridian-arc arithmetic, not taken from a published table
- independent: no
- inputs: a field 41.000° to 41.002° N; a 40 m fence; waypoints inside the field, 22.2 m due north of its north-west corner, and 66.6 m due north of it
- outputs: the first two are inside the fence; the third is outside by 26.6 m
- tolerance: 0.001 m
- verifiedBy: golden vector v001 and the `focused_mission_and_ops_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_drone.py` `geofence_check_vectors`: 19 random fields at six latitudes with waypoints set a known distance due north of a corner along its meridian, from the ellipsoid's meridional radius in Python; counts are exact and distances agree within 1 mm.
- `core/vectors/drone.mission.geofence-check.jsonl`: those vectors and two refusal cases.
- The integration suite requires the same counts and flagged list as `drone.mission.geofence`.

## Invariants

- A waypoint inside the area is never flagged.
- Widening the fence never adds an outside waypoint.
- Outside count + warning count ≤ waypoints checked.
