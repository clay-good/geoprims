<!-- Derivation note (trust/correctness-program layer A). -->
# Distance-distance intersection (`survey.cogo.distance-distance`)

## Method

The operation runs the circle-circle branch of the shared `survey.cogo.intersection` engine. Each known point is the center of a circle with the entered distance as its radius. The baseline locates the common chord, and the perpendicular half-chord gives the point on each side. Results are labeled left and right of the directed baseline from the first point to the second.

## Equations

For center separation d and radii r1 and r2, the chord foot is x = (r1² − r2² + d²)/(2d) from the first center. The half-chord is h = √(r1² − x²). Adding and subtracting h along the baseline's left normal gives the 2 solutions; h = 0 gives one tangent point.

## Symbols and units

The centers, d, x, h, r1, and r2 use one common linear unit. Left and right are viewed while facing from the first known point toward the second.

## Domain

The centers must be distinct plane coordinates, and both distances must be positive. The circles must touch or overlap without one lying wholly inside the other. International feet and US survey feet cannot be mixed.

## Approximations

None on the plane. Near-tangent circles magnify distance errors. Two intersections are geometrically valid, so field evidence must resolve the left-right ambiguity.

## Worked example

- sourcePublisher: GeoPrims project
- sourceTitle: add-survey-suite distance-distance geometry
- sourceEdition: 2026-10-13
- sourceLocator: intersections requirement
- independent: no
- inputs: 60 ft from (0 ft, 0 ft) and 40 ft from (0 ft, 100 ft)
- outputs: one tangent point at northing 0 ft, easting 60 ft
- tolerance: 1 part in 1 billion with a 1e-9 ft absolute allowance
- verifiedBy: golden vector v001
- verifiedOn: 2026-10-13

## Differential tests

- `core/vectors/survey.cogo.distance-distance.jsonl` contains 20 successful circle-circle cases evaluated independently in Python, plus separated-center, coincident-center, and mixed-foot refusal cases.

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `named_intersections_match_the_general_tool` requires exact parity with the circle-circle branch of `survey.cogo.intersection`.
- `core/crates/gp-survey/tests/survey.rs` `intersection_invariants` proves by inverse calculation that every answer is at both entered distances.
