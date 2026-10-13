<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Visual line of sight over a mission (`drone.ops.vlos-check`)

## Method

14 CFR 107.31, as printed on eCFR (read 2026-09-25), asks that the remote pilot in command and the person at the controls, or a visual observer, can see the aircraft with unaided vision throughout the flight, well enough to know its location, attitude, altitude, and direction, to watch for traffic and hazards, and to see that it endangers no one. It sets no distance. So the tool takes a visual range from the reader, or, from the drone's size, the EASA guidance distance the `drone.sensors.vlos` tool computes (the same code), and measures the geodesic distance from where the pilot stands to every waypoint. It reports the farthest waypoint, how many lie beyond the range and which ones, and a ring at the range for the map. Every result carries the dated not-legal-advice notice from `data/regulations.json` (row `faa-107-vlos`) and says that seeing the drone is judged on the day.

## Equations

- d_i = geodesic distance on WGS84 from the pilot to waypoint i (Karney 2013)
- Waypoint i is beyond when d_i > R + 1 µm; on the ring counts as inside
- Farthest = max d_i
- From the drone's size: R = min(ALOS, DLOS), ALOS = 327 × CD + 20 m (multirotor) or 490 × CD + 30 m (fixed wing), DLOS = 0.3 × ground visibility, counted to 5 km
- The ring: 72 points at R, every 5° of azimuth, by the direct geodesic problem

## Symbols and units

Pilot `lat`, `lon` in degrees; waypoints as latitude and longitude rows; R the visual range (m); CD the characteristic dimension (m). Distances come out in meters; waypoint numbers count from 1 in flight order.

## Domain

At least one waypoint; latitudes from −90° to 90°; a range over 0 and up to 50 km, or a characteristic dimension, not both.

## Approximations

Distances are horizontal only; height, terrain between the pilot and the drone, and obstacles are not considered. The 1 µm allowance covers a point placed on the ring with the direct problem and measured back with the inverse. The EASA distances are planning guidance, not a promise of sight.

## Worked example

- sourcePublisher: GeographicLib (C. F. F. Karney), GeodSolve 2.x command-line tool
- sourceTitle: Geodesic distances for a four-waypoint grid seen from 40° N, 105° W
- sourceEdition: GeodSolve installed 2026-09-25
- sourceLocator: `GeodSolve -i -p 9`
- independent: yes
- inputs: pilot at 40° N, 105° W; waypoints at 40.0009° N on 105° W and 104.98592° W, then 40.0027° N on 104.98592° W and 105° W; 500 m range
- outputs: 99.93, 1,206.48, 1,239.13, and 299.79 m; waypoints 2 and 3 beyond 500 m; the farthest is waypoint 3 at 1,239 m
- tolerance: 1e-9 relative on distances
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-25

## Differential tests

- `tools/vectors/gen_mission_planning.py`: 25 vectors cover GeodSolve distances at the equator, near a pole, in Colorado, London, Sydney, and across the antimeridian; metric and US range units; a point on the ring and another 1 cm past it; both EASA aircraft formulas; visibility limiting; and invalid range, coordinate, and input combinations.
- `core/crates/gp-drone/tests/planning.rs` `vlos_check_ring_and_dimension`: from a characteristic dimension the range equals `drone.sensors.vlos`'s VLOS distance, the ring's first point is that distance away, and the notice says "Not legal advice" and links § 107.31.

## Invariants

- A waypoint's `beyond` flag, the `beyond` list, and `beyond_count` agree, and the farthest distance is the largest in the distances list (checked in the vectors).
- `core/crates/gp-drone/tests/planning.rs` `vlos_check_lists_and_farthest_waypoint_are_consistent` checks those list and farthest-point relationships directly.
