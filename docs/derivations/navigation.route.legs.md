<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Route legs, courses, and totals (`navigation.route.legs`)

## Method

A list of waypoints becomes the table a navigation log is filled in from. Each consecutive pair is solved as an inverse geodesic problem, which gives the distance between them and the azimuth the route leaves on and arrives on. The distances accumulate into a running total, and with a groundspeed each leg's time follows, then the arrival clock time from the departure and the offset given.

## Equations

- Per leg: the inverse geodesic problem between waypoint i and i + 1 gives s₁₂, α₁, and α₂.
- True course of a leg is α₁ reduced to [0°, 360°); the final course is α₂, which differs from it because a geodesic's heading changes along the way.
- Running total after leg i: Σ s from the first leg to that one; the route's total is the last of these.
- Time for a leg: s / groundspeed. Arrival: departure + the total time, carried through the UTC offset given.

## Symbols and units

Waypoints are latitude and longitude in degrees, optionally named. Distances in nautical miles, courses in degrees true. Groundspeed accepts any speed unit. Times are written for a reader, "15 h 02 min", and the arrival is a clock time in the offset given.

## Domain

Two or more waypoints, anywhere, including across the antimeridian and over the poles, on any ellipsoid in the registry. Groundspeed and departure are optional: without them the table is distances and courses alone.

## Approximations

None in the geometry: each leg is the true geodesic, from Karney's solver, which is good to nanometres. The planning figures are another matter, and the tool says so — a single groundspeed is assumed to hold for the whole route, with no wind, climb, or descent in it, and the courses are true rather than magnetic.

## Worked example

- sourcePublisher: C. F. F. Karney, GeographicLib
- sourceTitle: GeodSolve, the inverse geodesic problem on WGS 84
- sourceEdition: GeographicLib version 2.7
- sourceLocator: `echo "39.8617 -104.6731 39.2232 -106.8688" | GeodSolve -i -p 6` gives azimuth −109.88409404290° and 201611.196182 m, which is 108.861337 nm on a course of 250.115906°; run for this note
- independent: yes
- inputs: Denver (39.8617, −104.6731) to Aspen (39.2232, −106.8688) as the first leg of a three-waypoint route
- outputs: leg 1 of 108.86133703130808 NM on a true course of 250.115905957101°
- tolerance: 1e-6 NM and 1e-6 degrees, which is the precision GeodSolve was asked to print
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-22

## Differential tests

- `tools/vectors/gen_route.py`: Karney's geographiclib for Python, solving the inverse problem per leg over random routes
- `core/vectors/navigation.route.legs.jsonl`: 22 vectors from that reference, run through the core on every build

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `route_legs_invariants`: over three routes, one of them across the antimeridian, there is one fewer leg than waypoints and `legs_count` agrees; the legs sum to the total and the last running total is the total; each leg's distance and course are the geodesic between its own two waypoints, checked against `navigation.geodesic.inverse` rather than against itself; flying the route backwards covers the same ground; and the written time is the distance at the speed given, to the minute it is rounded to
