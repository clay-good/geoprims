<!-- Derivation note (trust/correctness-program layer A). -->
# Bearing-distance intersection (`survey.cogo.bearing-distance`)

## Method

The operation runs the line-circle branch of the shared `survey.cogo.intersection` engine. A forward bearing or azimuth from the first known point defines a ray. The entered distance about the second known point defines a circle. Solving their quadratic returns every intersection in front of the first point, nearest first.

## Equations

For ray P(t) = P1 + t u and circle center C with radius r, solve |P1 + t u − C|² = r². The discriminant identifies no intersection, a tangent, or 2 crossings. Solutions with t < 0 are behind the entered direction and are discarded.

## Symbols and units

P1 and C are plane northing-easting coordinates in one length unit. The direction unit vector is u = (cos α, sin α), where α is clockwise from north. The radius r and parameter t use the coordinate unit.

## Domain

The first point supplies the forward direction, and the second point supplies a positive radial distance. All coordinates and the distance must use compatible linear units. International feet and US survey feet cannot be mixed.

## Approximations

None on the plane. A near-tangent crossing is sensitive to small input errors. When 2 points satisfy the observations, field evidence must identify the intended one.

## Worked example

- sourcePublisher: GeoPrims project
- sourceTitle: add-survey-suite bearing-distance scenario
- sourceEdition: 2026-10-13
- sourceLocator: intersections requirement
- independent: no
- inputs: a due-north ray from (0 m, 0 m) and a 50 m circle about (40 m, 30 m)
- outputs: 2 points, at northings 0 m and 80 m on easting 0 m
- tolerance: 1 part in 1 billion with a 1e-9 m absolute allowance
- verifiedBy: golden vector v001
- verifiedOn: 2026-10-13

## Differential tests

- `core/vectors/survey.cogo.bearing-distance.jsonl` contains 20 successful line-circle cases evaluated independently in Python, plus missed-circle, behind-direction, and mixed-foot refusal cases.

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `named_intersections_match_the_general_tool` requires exact parity with the line-circle branch of `survey.cogo.intersection`.
- `core/crates/gp-survey/tests/survey.rs` `intersection_invariants` uses inverse calculations to prove every result lies on both the entered direction and circle.
