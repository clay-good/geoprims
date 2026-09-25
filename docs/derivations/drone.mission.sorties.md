<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Mission sorties and battery swaps (`drone.mission.sorties`)

## Method

The tool walks the waypoint path in order and cuts it into sorties, one battery each. A sortie that starts at waypoint s flies out from home to s at the transit speed, then along the path at the survey groundspeed. At each waypoint j it asks what the trip home would take from there, and it ends the sortie at the last waypoint where the time flown plus the time home still fits in the battery's time less the reserve. The next battery flies out to that waypoint and carries on from it. No formula is new: the time per battery is the endurance tool's (`power::usable_energy`, usable energy × (1 − reserve), divided by the power), and each trip home is the return-to-home tool's (`power::return_home`, distance / (airspeed − headwind)). Distances are WGS84 geodesics (Karney 2013). A waypoint that a full battery cannot reach and come home from, or a leg that one battery cannot fly out to, along, and back from, stops the plan with NO_SOLUTION naming that waypoint and its distance.

## Equations

- Time per battery F = T × (1 − r), or F = E × u × (1 − r) / P from a battery energy
- Elapsed at the start of a sortie at waypoint s: t = d_s / V_t
- Elapsed at the next waypoint: t ← t + L_j / V_c
- Trip home from waypoint j: h_j = d_j / (V_t − w)
- The sortie ends at the last j with t_j + h_j ≤ F; flying time = t_j, return time = h_j, return energy = P × h_j

## Symbols and units

T flight time per battery (min); r reserve share; E battery energy (Wh), u usable share, P cruise power (W); d_j geodesic distance from home to waypoint j and L_j the leg from j to j + 1 (m); V_c survey groundspeed and V_t transit airspeed (m/s, V_t defaults to V_c); w headwind on the way home (m/s). Out come `batteries`, `total_time` and `battery_time` (min, to 0.1 min), the `sorties` rows, the `swap_points`, and the `path` rows (home, the sortie's waypoints, home) with a `part` number per sortie.

## Domain

At least one waypoint; latitudes from −90° to 90°; positive speeds; a flight time, or a battery energy with a power, not both. A headwind at least as fast as the transit speed cannot be flown home against and is refused.

## Approximations

Every leg is flown at constant speed and power with no time for takeoff, climb, turns, or landing. The wind is one headwind on every trip home and none on the way out or along the path, which is cautious on the way home and ignores the rest. A sortie can end only at a waypoint, so on long legs it can stop short of what the battery could reach. The time on the clock is compared with a relative slack of 1e-12 so round-off in the last digit does not end a sortie early.

## Worked example

- sourcePublisher: geoprims (hand-worked; no published sortie example was found)
- sourceTitle: A four-line survey grid on a 7-minute battery, worked by hand below
- sourceEdition: distances from GeographicLib GeodSolve, 2026-09-25
- sourceLocator: this section
- independent: yes, for the walk (distances from GeodSolve, arithmetic by hand)
- inputs: home 40° N, 105° W; lines at 40.0009°, 40.0027°, 40.0045°, and 40.0063° N from 105° W to 104.98592° W, flown as a serpentine (8 waypoints); 7 min battery, 20% reserve, 10 m/s
- outputs: 3 batteries, swapping at waypoints 4 and 7, 16.0 min in all
- tolerance: 1e-9 relative on times
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-25

GeodSolve gives the distances from home to waypoints 1 to 8 as 99.9, 1,206.5, 1,239.1, 299.8, 499.7, 1,302.0, 1,391.0, and 699.5 m, and the legs as 1,202.3 m along each line and 199.9 m between lines. The time per battery is 7 min × 0.8 = 336 s. At 10 m/s:

| Sortie | At waypoint | Time flown (s) | Time home (s) | Sum (s) | Fits 336 s? |
|---|---|---|---|---|---|
| 1 | 1 | 10.0 | 10.0 | 20.0 | yes |
| 1 | 2 | 10.0 + 120.2 = 130.2 | 120.6 | 250.9 | yes |
| 1 | 3 | 130.2 + 20.0 = 150.2 | 123.9 | 274.1 | yes |
| 1 | 4 | 150.2 + 120.2 = 270.4 | 30.0 | 300.4 | yes |
| 1 | 5 | 270.4 + 20.0 = 290.4 | 50.0 | 340.4 | no: swap at 4 |
| 2 | 4 | 30.0 | 30.0 | 60.0 | yes |
| 2 | 5 | 50.0 | 50.0 | 100.0 | yes |
| 2 | 6 | 170.2 | 130.2 | 300.4 | yes |
| 2 | 7 | 190.2 | 139.1 | 329.3 | yes |
| 2 | 8 | 310.4 | 70.0 | 380.4 | no: swap at 7 |
| 3 | 7 | 139.1 | 139.1 | 278.2 | yes |
| 3 | 8 | 259.3 | 70.0 | 329.3 | yes: done |

In all, 300.4 + 329.3 + 329.3 = 959.0 s, 16.0 min, on 3 batteries. Dividing the path alone by the battery time, 5,408.9 m / 10 m/s = 541 s over 336 s, says 2: the trips out and home are what add the third.

## Differential tests

- `tools/vectors/gen_mission_planning.py` walks the same rule in Python from GeographicLib GeodSolve distances (a separate implementation from the core's Rust port) for five paths: the worked example, a path one battery covers, a 2 km-long grid with a separate transit speed and a headwind home, a battery energy and power in place of a flight time, and speeds in km/h, plus a waypoint 17 km out that no battery can reach (NO_SOLUTION at `/waypoints/1`).
- `core/crates/gp-drone/tests/planning.rs` `sorties_compose_endurance_and_rth`: the battery time equals `drone.power.endurance`'s time to the reserve, and every sortie's return time and energy equal `drone.power.rth-budget`'s for the same distance, airspeed, headwind, and power.

## Invariants

- `core/crates/gp-drone/tests/planning.rs` `sorties_far_ends_are_shorter`: on a grid whose far end is 2 km from home and 20 min batteries, the sortie that ends farthest out flies less than the one that ends nearest home.
- `sorties_hand_worked_three`: every sortie's time flown plus time home fits within the battery time less the reserve.
- `sorties_one_battery_when_it_fits` and `sorties_waypoint_out_of_range`: a path within one battery takes one, and an unreachable waypoint is named with its distance.
