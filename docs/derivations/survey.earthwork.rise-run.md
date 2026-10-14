<!-- Derivation note (trust/correctness-program layer A). -->
# Rise, run, and slope length (`survey.earthwork.rise-run`)

## Method

A straight slope is a right triangle: the run is horizontal, the rise is vertical, and the slope length is the hypotenuse. Any two sides give the third, and the rise over the run is the grade. The operation takes two of the three lengths and reports all three with the grade. It shares one calculation path with `survey.earthwork.grade`, which also reads a grade written as a percent, ratio, or angle.

## Equations

- g = rise / run
- slope length = √(rise² + run²)
- run = √(slope² − rise²) from a rise and a slope length
- rise = √(slope² − run²) from a run and a slope length
- percent = 100·g, per mille = 1,000·g, angle = atan g, ratio H:V = (1/g):1, V:H = g:1

## Symbols and units

Rise, run, and slope length share one length unit: meters, international feet, or US survey feet. A rise is positive uphill. The angle is in degrees from the horizontal.

## Domain

Two of the three lengths are required. The run and the slope length must be positive, and a slope length must be longer than the other side given. Mixing international and US survey feet is refused with UNIT_MISMATCH.

## Approximations

None in the arithmetic. A rise computed from a run and a slope length is reported as positive, because those two lengths do not say which way the ground falls.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: add-survey-suite specification
- sourceEdition: 2026
- sourceLocator: survey/earthwork-and-grade, requirement "Grade and slope expressions" (rise, run, or slope length from any two); the 3-4-5 triangle is checked by hand, not taken from a published table
- independent: no
- inputs: rise 3 m, slope length 5 m
- outputs: run 4 m, grade 75%, angle 36.8699°
- tolerance: 1e-12
- verifiedBy: golden vectors v002 and v003 and the `named_earthwork_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_survey.py` `rise_run`: 17 random triangles in all three length units, each solved from a different pair of sides by a separate Python evaluation.
- `core/vectors/survey.earthwork.rise-run.jsonl`: those vectors, three hand-checked cases, and four refusal cases.
- The integration suite requires the same grade and lengths as `survey.earthwork.grade` for the same inputs.

## Invariants

- rise² + run² = slope length².
- The grade from a rise and a run does not depend on the unit they share.
- Each pair of sides of the same triangle returns the same third side.
