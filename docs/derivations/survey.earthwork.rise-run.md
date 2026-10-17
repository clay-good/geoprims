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

- sourcePublisher: United States Access Board
- sourceTitle: 2010 ADA Standards for Accessible Design
- sourceEdition: 2010
- sourceLocator: Section 405.2, ramp running slope maximum 1:12 (8.33%)
- independent: yes
- inputs: rise 2.5 ft (30 in), run 30 ft
- outputs: grade 8.333%, ratio 12H:1V, angle 4.7636°, slope length 30.104 ft
- tolerance: the standard's 1:12 and 8.33% match exactly; the core reports 8.3333% and 12H:1V
- verifiedBy: golden vector v001 and the `named_earthwork_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-17

## Differential tests

- `tools/vectors/gen_survey.py` `rise_run`: 17 random triangles in all three length units, each solved from a different pair of sides by a separate Python evaluation.
- `core/vectors/survey.earthwork.rise-run.jsonl`: those vectors, three hand-checked cases, and four refusal cases.
- The integration suite requires the same grade and lengths as `survey.earthwork.grade` for the same inputs.

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `rise_run_invariants`: rise² + run² = slope length², each pair of sides of the same triangle returns the same third side, and the grade from a rise and a run does not depend on the unit they share.
