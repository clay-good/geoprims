<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Closest point of approach (`navigation.route.cpa`)

## Method

Set the first object at the origin of a local east-north frame and give the second its position relative to it. If both hold course and speed, the vector between them changes at a constant rate, so their separation as a function of time is the length of a straight line traced at constant velocity — a parabola in time, with one minimum. Differentiating and setting to zero gives the time of closest approach in closed form; substituting it back gives the separation, and the bearing and range at that moment. With heights and climb rates the same argument runs in three dimensions with a third component, and nothing else changes. If the minimum lies in the past the two are already separating; the time is reported as zero and the `DIVERGING` warning says so, since a negative time is not a moment anyone can act on and the present separation is what matters.

## Equations

- Relative position r = B − A and relative velocity v = v_B − v_A, in the local frame.
- Time of closest approach: t = max(0, −(r · v) / |v|²), undefined when |v| = 0 (they hold station on each other); a negative value means the approach has passed and is reported as zero with a warning.
- Separation then: |r + v t|.
- Bearing at that moment: the compass bearing of r + v t, and the range its length.
- In three dimensions the same expressions with a third component, heights differencing and climb rates entering v.

## Symbols and units

r is the position of the second object relative to the first, east and north (and up) in meters; v their relative velocity in meters per second; t in seconds, never negative: zero when the closest approach has passed. Courses are true bearings in degrees, speeds in the unit given. `scene_end` is how far the timeline is drawn, not part of the answer.

## Domain

Any two constant velocities. The frame is flat and local, so the two should be within about 500 km of each other, where treating the Earth as a plane costs about a tenth of a percent. Equal velocities make |v| zero and the separation constant, which has no single closest moment. A closest approach in the past is flagged `DIVERGING` and the present separation carries the answer.

## Approximations

One, and it is the frame: a plane standing in for the ellipsoid over the distance between the two objects. Within 500 km that is good to about 0.1%. The rest is exact — the time and the separation are closed forms, not a search — and the constant-velocity assumption is not an approximation of the geometry but a statement about the future, which no arithmetic can improve.

## Worked example

- sourcePublisher: elementary relative-motion geometry, with every output an exact closed form
- sourceTitle: the closest approach of two points at constant velocity
- sourceEdition: t = −(r·v)/|v|², separation |r + vt|
- sourceLocator: r = (1000, 1200) m, v = (−10, −10) m/s, chosen so that every answer is exact
- independent: yes
- inputs: A on course 090° at 10 m/s; B 1000 m east and 1200 m north, on course 180° at 10 m/s
- outputs: t = 110 s, separation 100√2 = 141.4213562373095 m, bearing 315°, present separation √(1000² + 1200²) = 1562.0499351813308 m
- tolerance: 1e-12 relative, which is where the arithmetic runs out
- verifiedBy: golden vector v027, run by the core on every build
- verifiedOn: 2026-09-23

This example is deliberately built so that no reference implementation is needed and none could be more authoritative than the arithmetic itself. The relative velocity is (−10, −10) m/s, so |v|² = 200 and r · v = −22000, giving t = 22000/200 = 110 s with no rounding anywhere. At that moment the relative position is (1000 − 1100, 1200 − 1100) = (−100, +100) m: due north-west, so the bearing is exactly 315°, and the separation is 100√2, whose double-precision value is 141.4213562373095. The tool returns 110.00000000000001 s, 141.42135623730934 m, and 314.99999999999994° — each the correct value to the last place or two of a double.

An exactly-stated answer is a weaker check than a second implementation would be, because it exercises one point rather than a range. That is what the differential test below is for: it solves the same problem by a different method, over hundreds of cases.

## Differential tests

- `core/crates/gp-navigation/tests/navigation.rs` `cpa_matches_a_brute_force_search`: 300 random pairs of courses, speeds and offsets, with the minimum separation found by golden-section search over time rather than by the closed form — a different algorithm for the same problem, which a sign or a factor in the closed form could not survive. Over the 148 pairs that are actually closing, the separations agree to 7.3e-12 m. The moment is held only to a millisecond and deliberately so: the separation is flat at its minimum, so a search on distance finds the value precisely and the argument loosely, and a tight bound on the time would be measuring the search rather than the tool
- `tools/vectors/gen_route.py`: 26 vectors from relative motion evaluated in Python, including the three-dimensional cases and the diverging ones
- `core/vectors/navigation.route.cpa.jsonl`: 27 vectors, the last this worked example

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `cpa_invariants`: the answer is symmetric — swapping which object is called A leaves the time and the separation unchanged, the bearing reversed; the separation at the reported time is not exceeded anywhere else on the track, checked by sampling around it; two objects on the same course at the same speed never close, and their separation is reported as constant; a pair set up to have already passed each other raises `DIVERGING` and reports a time of zero rather than a negative one — the closest approach is behind them, so the useful answer is the present separation, and the tool clamps instead of offering a moment that cannot be flown to; in three dimensions the horizontal and vertical separations recompose to the total; and giving both the same height with no climb reproduces the two-dimensional answer exactly, which is the check that the third dimension is entering only where it should
