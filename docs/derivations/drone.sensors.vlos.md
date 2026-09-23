<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# VLOS distance, EASA guidance (`drone.sensors.vlos`)

## Method

EASA's acceptable VLOS distance: the attitude line of sight (ALOS), the distance at which the remote pilot can still see the drone's position and orientation, from its characteristic dimension; the detection line of sight (DLOS), the distance at which other aircraft can be seen in time to avoid them, from the ground visibility; and VLOS as the smaller of the two. With a planned farthest point, the tool reports whether the mission stays within VLOS and the margin. US Part 107 sets no numeric VLOS distance, and the tool says so.

## Equations

- ALOS = 327 × CD + 20 m (rotorcraft and multirotor), 490 × CD + 30 m (fixed wing).
- DLOS = 0.3 × GV.
- VLOS = min(ALOS, DLOS); margin = VLOS − farthest distance.

## Symbols and units

CD characteristic dimension (the largest dimension of the aircraft, m), GV ground visibility (km; EASA recommends at least 5 km, and the LBA guidance counts at most 5 km), distances in m.

## Domain

A positive characteristic dimension and ground visibility. EASA's guidelines say the ground visibility "should be at least 5 km", a minimum, so a visibility below 5 km gets a VISIBILITY_BELOW_MINIMUM caution (the distance is still computed from it). The LBA guidance EASA's footnote points to sets GVmax = 5 km, so a larger value is counted as 5 km with a note and VLOS never exceeds 1,500 m. Without a visibility, 5 km is used and the tool says so.

## Approximations

This is EASA guidance for planning, not a physical visibility model. Lighting, background, color, and the pilot's eyesight change what can actually be seen, and the result is labeled as guidance, not a legal limit.

## Worked example

- sourcePublisher: Luftfahrt-Bundesamt (German Federal Aviation Office)
- sourceTitle: Guidance for Dimensioning of Flight Geography, Contingency Volume and Ground Risk Buffer (referenced by EASA's guidelines, Issue 03, footnote 3)
- sourceEdition: English edition, retrieved 2026-09-19
- sourceLocator: Section 7.1, maximum VLOS distance for ground visibility of 5 km or more: CD 1 m 347 m (rotary) and 520 m (fixed wing); 2 m 674 m and 1,010 m; 3.5 m 1,164.5 m; 4 m 1,328 m; 4.53 m and over 1,500 m. The 3 m rotary-wing row prints 1,000 m where the formula gives 1,001 m, so it is not used.
- independent: yes
- inputs: characteristic dimension 1 m, multirotor, ground visibility 5 km
- outputs: VLOS 347 m
- tolerance: exact
- verifiedBy: golden vectors v012 to v019, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `tools/vectors/gen_drone.py`: a separate Python evaluation of the EASA procedure at 11 aircraft, including 6 random sizes, types, and visibilities (within 1e-9 relative)
- `core/vectors/drone.sensors.vlos.jsonl`: those vectors, the LBA table, farthest-point margins, and the 1,500 m cap (v022, v023), run through the core on every build

## Invariants

- `core/crates/gp-drone/tests/power_ops.rs` `vlos_invariants`: VLOS is the smaller of ALOS and DLOS, ALOS grows by 327 or 490 per meter of size, DLOS is 0.3 × ground visibility, and the margin is VLOS minus the farthest point; with no visibility or one above 5 km, VLOS stops at 1,500 m
