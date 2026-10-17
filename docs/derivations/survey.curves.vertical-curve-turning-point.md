<!-- Derivation note (trust/correctness-program layer A). -->
# Vertical curve high or low point (`survey.curves.vertical-curve-turning-point`)

## Method

An equal-tangent vertical curve is a parabola whose grade changes at a constant rate from g1 at the PVC to g2 at the PVT. Where the grade passes through zero the curve is at its highest or lowest. The operation finds that point and its elevation. It shares one calculation path with `survey.curves.vertical-curve`.

## Equations

With grades as ratios and x measured from the PVC:

- grade(x) = g1 + (g2 − g1)·x / L
- x = −g1·L / (g2 − g1) where the grade is zero
- elevation(x) = PVC elevation + g1·x + (g2 − g1)·x² / (2L)
- PVC elevation = PVI elevation − g1·L / 2
- K = L / |g2 − g1| with grades in percent

## Symbols and units

g1, g2 grades in percent, up positive; L curve length; stations as 12+34.56 in feet or 1+234.567 in meters. The length and elevation share one unit.

## Domain

A positive length (or K value) and two different grades. The point exists only when the grades have opposite signs; otherwise the operation refuses with OUT_OF_DOMAIN and says the extremes are the curve's ends.

## Approximations

None for a parabolic curve. Unequal-tangent curves are a different operation.

## Worked example

- sourcePublisher: Engineering LibreTexts (David Levinson et al., CC BY-SA 4.0)
- sourceTitle: Fundamentals of Transportation, 7.5 Vertical Curves, Example 1 (sag low point)
- sourceEdition: CC BY-SA 4.0
- sourceLocator: G1 −4%, G2 +2%, L 500 m, PVC 100+00 at 1000 m (PVI 102+50 at 990 m)
- independent: yes
- inputs: g1 −4%, g2 +2%, L 500 m, PVI 102+50 at 990 m
- outputs: low point 333.33 m past the PVC, elevation 993.33 m (the core renders the station in km-chunks as 10+333.333, where the source adds 333.33 m to 100+00 for 103+33.33)
- tolerance: the source's printed 0.01 m
- verifiedBy: golden vector v024 and the `named_curve_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-17

## Differential tests

- `tools/vectors/gen_survey.py` `vertical_curve_parts`: 18 random crest and sag curves in all three length units, evaluated in exact fractions.
- `core/vectors/survey.curves.vertical-curve-turning-point.jsonl`: those vectors, two hand-checked cases, and three refusal cases.
- The integration suite requires the same station, elevation, and K as `survey.curves.vertical-curve`.

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `vertical_curve_turning_point_invariants`: the grade is zero at the turning point (g1 + (g2 − g1)·x/L = 0), its distance past the PVC is K·|g1| and lies between 0 and L, and a rise-then-fall grade reports a crest (high) while a fall-then-rise reports a sag (low).
