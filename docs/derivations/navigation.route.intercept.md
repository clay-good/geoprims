<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Intercept a moving target (`navigation.route.intercept`)

## Method

The target keeps a steady course and speed, so at any future time its position is known: run its course from where it is now, for its speed times the time, along a geodesic. You fly straight (along a geodesic) to wherever you will meet it. You meet it at the first time when the distance from your starting point to the target's position then equals the distance you can have flown by then.

So the tool looks for the first time t at which f(t), the range to the target at t minus your run v·t, reaches zero. It steps forward in time by f divided by your speed plus the target's: f can shrink no faster than that combined speed, so such a step can never jump past the first zero. When a Newton step lands on the far side of zero, it closes in on it between the two by regula falsi. The meeting point, your course to it, and your run then follow from the geodesic problems.

If the target runs half the Earth's girth, or you run 40,000 km, without a meeting, the target is reported as unreachable, with the opening rate when it is simply running away faster than you can close.

## Equations

- Target: T(t) = direct(T₀, course_T, v_T·t), the geodesic direct problem.
- Range: r(t) = inverse(P, T(t)), the geodesic distance from your start P.
- Meeting: the first t ≥ 0 with f(t) = r(t) − v·t ≤ 1 mm.
- Step: t ← t + f(t)/(v + v_T), which cannot pass the first root since |f′| ≤ v + v_T; a Newton step t − f/f′ that brackets the root is finished by regula falsi.
- Result: the course and your run from inverse(P, T(t)); the time t; the meeting point T(t).

## Symbols and units

Inputs: your position (`lat`, `lon`, degrees) and speed over the ground (`speed`); the target's position (`target_lat`, `target_lon`), course (`target_course`, degrees true), and speed over the ground (`target_speed`). Outputs: `course` to steer (the initial geodesic course, degrees true), `time` to the meeting (minutes), `distance`, your run (meters), and the meeting point (`meet_lat`, `meet_lon`).

## Domain

WGS 84. Your speed above zero, the target's zero or more; any positions and course. The search horizon is the target running half the Earth's girth or you running 40,000 km, whichever comes first.

## Approximations

None in the solution for steady motion: it meets to a millimeter of range. The assumption is the steady motion itself, and speeds over the ground.

## Worked example

- sourcePublisher: C. F. F. Karney, GeographicLib (Python package), with a solver written for this check
- sourceTitle: first root of range minus run, on a fixed time grid then bisection
- sourceEdition: geographiclib 2 (Python)
- sourceLocator: `tools/vectors/gen_intercept_more.py`: f(t) on a grid of 0.01 h out to the same horizon, the first sign change, then 80 bisections
- independent: yes
- inputs: 200 cases anywhere from 70° S to 70° N, targets 1 to 150 NM away on random bearings, your speed 5 to 40 kt against the target's 0 to 30 kt on random courses; 59 of them unreachable
- outputs: the time to the meeting, your run, the course, and the meeting point, or no meeting
- tolerance: 1e-6 h on the time and 2 cm on the run
- verifiedBy: `core/crates/gp-navigation/tests/intercept.rs` `intercept_matches_a_grid_solver`; golden vectors v018 to v027
- verifiedOn: 2026-09-24

The reference finds the same first root in a different way, by walking a fixed grid rather than the tool's step that cannot overshoot, with GeographicLib's Python package rather than the Rust port the tool uses. It agrees on all 200: the same 141 meetings, the times within 0.09 ms and the runs within 0.23 mm at worst, and the same 59 targets that cannot be reached. Cases where the range comes within 50 m of your run before the first crossing, a near-graze where which crossing is "first" depends on resolution, are left out.

## Differential tests

- `tools/vectors/gen_intercept_more.py`: the fixture and vectors v018 to v027; v001 to v017 came earlier, among them bisection with GeodSolve in `tools/vectors/gen_intersect.py`
- `core/crates/gp-navigation/tests/intercept.rs` `intercept_matches_a_grid_solver`: all 200 cases

## Invariants

- `core/crates/gp-navigation/tests/intercept.rs` `intercept_invariants`: your run is your speed times the time; flying the reported course for the run and the target running its course for the time both arrive at the meeting point, through `navigation.geodesic.direct`; a faster pursuer meets sooner; a stationary target is met after exactly the geodesic distance to it, from `navigation.geodesic.inverse`; and a target running straight away faster than you is refused with NO_SOLUTION.
