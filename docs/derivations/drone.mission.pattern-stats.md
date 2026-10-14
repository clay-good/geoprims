<!-- Derivation note (trust/correctness-program layer A). -->
# Flight path statistics (`drone.mission.pattern-stats`)

## Method

A mission is a list of waypoints flown in order. Each leg is the geodesic, the shortest path on the WGS 84 ellipsoid, between two consecutive waypoints. The path length is the sum of the legs. At each waypoint between legs, the course changes from the azimuth the last leg arrives on to the azimuth the next leg leaves on; that change, taken the short way round, is the turn there.

## Equations

- Leg i: (s_i, α1_i, α2_i) = geodesic inverse from waypoint i to waypoint i + 1 (Karney 2013)
- Path length = Σ s_i; longest leg = max s_i
- Course change at waypoint i + 1: Δ_i = |((α1_(i+1) − α2_i + 540) mod 360) − 180|
- Turns = the number of Δ_i over 5°; total turning = Σ Δ_i
- Farthest from the first waypoint = max over waypoints of the geodesic distance from waypoint 1
- Flight time = path length ÷ groundspeed

## Symbols and units

Waypoints in latitude and longitude on WGS 84; s leg length in meters; α1 and α2 the departure and arrival azimuths of a leg in degrees; the path length is reported in kilometers and the flight time in minutes.

## Domain

Two to 5,000 waypoints with latitudes from −90° to 90°. Two consecutive waypoints may not be the same point, since a leg of no length has no course. A groundspeed, when given, must be positive.

## Approximations

Legs are straight over the ground at no particular height: climbs, descents, turn radius, and time spent hovering are not included. The 5° threshold for counting a turn is a convention of this tool, stated in the result's field help.

## Worked example

- sourcePublisher: Charles F. F. Karney
- sourceTitle: GeographicLib, GeodSolve utility
- sourceEdition: GeographicLib 2.7
- sourceLocator: `GeodSolve -i -p 9` on each leg of the path 44.98, −93.27 → 44.9818, −93.27 → 44.9818, −93.2662 → 44.98, −93.2662 → 44.98, −93.27
- independent: yes
- inputs: those five waypoints at 8 m/s
- outputs: path length 0.999508057 km in 4 legs, longest leg 299.722 m, farthest waypoint 360.340 m from the first, 3 turns, 2.0823 min
- tolerance: 1e-6 m on lengths
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_pattern_stats.py`: every leg of 20 paths of 1 to 12 legs, at eight sites from 55° S to 78° N, solved by the GeodSolve command-line program rather than the Rust port of GeographicLib the core links, and summed in Python.
- `core/vectors/drone.mission.pattern-stats.jsonl`: those vectors and three refusal cases.

## Invariants

- `core/crates/gp-drone/tests/drone.rs` `pattern_stats_invariants`: a path flown backwards has the same length and turns; a straight line has no turns; an out-and-back has one 180° turn; splitting a leg at a point on it adds a leg and no length; flight time × groundspeed = path length.
