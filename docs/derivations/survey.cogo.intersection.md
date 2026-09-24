<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Intersection (`survey.cogo.intersection`)

## Method

Plane coordinate geometry from two known points, each with a direction or a distance. Two directions give two lines, solved as a pair of linear equations. A direction and a distance give a line and a circle, solved as a quadratic along the line. Two distances give two circles, solved on the baseline between their centers. Every answer is returned and labeled. Circle-circle answers are labeled left or right of the baseline from the first point to the second; line-circle answers are ordered along the line from its point. Lines that are parallel, or that meet behind either point, and circles that do not meet, are refused.

## Equations

- Line-line: P₁ + t·u₁ = P₂ + s·u₂, with u = (sin az, cos az) in (easting, northing); the answer needs t > 0 and s > 0
- Line-circle: |P₁ + t·u₁ − P₂|² = r₂², giving t = p ± √(r₂² − q²), with p the projection of P₂ − P₁ on u₁ and q its perpendicular offset
- Circle-circle: with d = |P₂ − P₁|, x = (r₁² − r₂² + d²) / (2d) along the baseline and h = √(r₁² − x²) either side of it

## Symbols and units

N northing and E easting in one length unit; az an azimuth or bearing; r a distance in the same unit.

## Domain

Two distinct known points. The inputs for each point are a direction or a distance, not both. The problem must have at least one answer.

## Approximations

None on the plane. With a shallow crossing angle, small errors in the directions move the answer a long way.

## Worked example

- sourcePublisher: Michigan Department of Transportation
- sourceTitle: Survey manual, part III, Basic Survey Observations
- sourceEdition: retrieved 2026-09-24
- sourceLocator: Section 3.6.1 and figure 3.26, the line-line intersection example: A (X 5447.330, Y 4080.822) on azimuth 334°48′47″ and B (X 5752.796, Y 4377.864) on 308°39′58″
- independent: yes
- inputs: those two points and azimuths
- outputs: C at X 5039.038, Y 4948.999 (with D_AB = 426.079 and D_BC = 914.136 along the way)
- tolerance: 0.001 (the printed rounding)
- verifiedBy: golden vector v007, run by the core on every build
- verifiedOn: 2026-09-24

## Differential tests

- `tools/vectors/gen_survey.py`: each case solved in closed form in Python at random state-plane-sized points: five line-line intersections toward a point ahead of both, five circle-circle pairs with both labeled answers, and five line-circle crossings with both answers ahead of the start (within 1e-9 relative)
- `core/vectors/survey.cogo.intersection.jsonl`: those vectors, the published example, the spec scenarios, and the refusals for parallel lines, lines crossing behind a point, and circles that do not meet, run through the core on every build

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `intersection_invariants`: each answer lies on what defined it (the inverse from each known point gives back its azimuth or distance); shifting the problem shifts the answer; and swapping the two points leaves a line-line answer where it was
