<!-- Derivation note (trust/correctness-program layer A). -->
# Bearing-bearing intersection (`survey.cogo.bearing-bearing`)

## Method

The operation runs the line-line branch of the shared `survey.cogo.intersection` engine. Each entered bearing or azimuth becomes a forward unit vector from its known point. The 2 parametric lines are solved together, and the result is accepted only when it lies forward of both points.

## Equations

For known points P1 and P2 and direction vectors u1 and u2, solve P1 + t u1 = P2 + s u2. A zero determinant means the directions are parallel. Negative t or s means the lines cross behind an entered direction.

## Symbols and units

P is a plane coordinate in one common length unit. A north azimuth α gives u = (cos α, sin α) in northing-easting order. The parameters t and s are forward distances in that unit.

## Domain

The 2 known points use one plane grid or local coordinate system. Each direction is a quadrant bearing, azimuth, or gon value and must point toward the wanted intersection. International feet and US survey feet cannot be mixed.

## Approximations

None on the plane. A shallow crossing angle magnifies small direction errors, and the 2 observations provide no redundancy or blunder detection.

## Worked example

- sourcePublisher: Michigan Department of Transportation
- sourceTitle: Survey manual, part III, Basic Survey Observations
- sourceEdition: retrieved 2026-09-24
- sourceLocator: Section 3.6.1 and figure 3.26
- independent: yes
- inputs: A at northing 4080.822 ft, easting 5447.330 ft on azimuth 334°48′47″; B at northing 4377.864 ft, easting 5752.796 ft on azimuth 308°39′58″
- outputs: C at northing 4948.999 ft, easting 5039.038 ft
- tolerance: 0.001 ft for the printed coordinates
- verifiedBy: golden vector v001
- verifiedOn: 2026-10-13

## Differential tests

- `core/vectors/survey.cogo.bearing-bearing.jsonl` contains the published example, 19 successful independent closed-form cases, and parallel, backward, and mixed-foot refusal cases.

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `named_intersections_match_the_general_tool` requires this operation to return the exact same result, display values, and warnings as the line-line branch of `survey.cogo.intersection`.
- `core/crates/gp-survey/tests/survey.rs` `intersection_invariants` checks both output bearings by inverse calculation and proves point-order and coordinate-shift invariance.
