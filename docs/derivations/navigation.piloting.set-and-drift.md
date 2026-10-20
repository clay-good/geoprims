<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Current triangle (set and drift) (`navigation.piloting.set-and-drift`)

## Method

The vector triangle ground = water + current, in the three forms of Bowditch §1008. (1) Track: the water vector (course, speed) plus the current vector (set, drift). (2) Course to steer: the heading whose water vector plus the current lies along the course to make good, at the given speed. (3) Course and speed to use: the water vector equals the required ground vector less the current. The three run on `gp_geo::wind`'s ground_vector, heading_groundspeed, and air_vector, the functions the aviation wind tools use, with the current's set (toward) turned into a wind's from direction by adding 180°.

## Equations

- Track: (t, g) = polar(water + current).
- Course to steer: with the current taken as a wind from set + 180°, the correction δ = asin(d sin(set + 180° − C) / v), the course to steer C + δ, and the speed made good v cos δ − d cos(set + 180° − C).
- Course and speed: (h, w) = polar(ground − current).

## Symbols and units

Courses and set in degrees true; speed, drift, and speed made good in knots.

## Domain

A positive speed and a drift of zero or more. Course to steer has no solution when the current across the track exceeds the ship's speed, or when along the track it carries the ship backward (`NO_SOLUTION`).

## Approximations

A steady current; Bowditch solves by plotting, to about 0.5° and 0.1 kn.

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: The American Practical Navigator (Bowditch), Pub. No. 9
- sourceEdition: 2024 edition
- sourceLocator: §1008, examples 1 to 3
- independent: yes
- inputs: Course 080° 10 kn, set 140° drift 2 kn; make good 095° at 12 kn in set 170° drift 2.5 kn; make good 265° at 15 kn in set 185° drift 3 kn
- outputs: Track 089°, 11.2 kn; steer 083.5°, 12.4 kn; steer 276°, 14.8 kn (plotted and printed to the degree; the exact triangle gives 276.53°)
- tolerance: 0.5° and 0.1 kn (1° on the third, printed to the degree)
- verifiedBy: golden vector v001 and `core/crates/gp-navigation/tests/piloting.rs`
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_piloting.py set-and-drift`: Python vector arithmetic at 17 random cases across the three problems (1e-9) and the too-strong-current refusal.
- `core/vectors/navigation.piloting.set-and-drift.jsonl`: those vectors, run through the core on every build.

## Invariants

- `core/crates/gp-navigation/tests/piloting.rs` `current_triangle_bowditch_1008`: the three examples and the refusals; `marine_and_aviation_triangles_agree`: the course to steer equals the aviation wind triangle's heading on the same vectors, bit for bit.
