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

- sourcePublisher: geoprims
- sourceTitle: add-survey-suite specification
- sourceEdition: 2026
- sourceLocator: survey/alignment-curves, scenario "Crest curve high point"; the arithmetic is checked by hand, not taken from a published table
- independent: no
- inputs: g1 +2%, g2 −3%, L 600 ft, PVI 10+00 at 100.00 ft
- outputs: high point 240 ft past the PVC, station 9+40, elevation 96.40 ft, K 120
- tolerance: 1e-9 ft
- verifiedBy: golden vector v002 and the `named_curve_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_survey.py` `vertical_curve_parts`: 18 random crest and sag curves in all three length units, evaluated in exact fractions.
- `core/vectors/survey.curves.vertical-curve-turning-point.jsonl`: those vectors, two hand-checked cases, and three refusal cases.
- The integration suite requires the same station, elevation, and K as `survey.curves.vertical-curve`.

## Invariants

- The distance past the PVC is between 0 and L.
- A crest's high point is above both ends; a sag's low point is below both.
- Reversing the curve (swapping and negating the grades) mirrors the point about the PVI.
