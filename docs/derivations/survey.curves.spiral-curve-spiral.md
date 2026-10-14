<!-- Derivation note (trust/correctness-program layer A). -->
# Spiral-curve-spiral stations (`survey.curves.spiral-curve-spiral`)

## Method

A spiraled curve eases from the tangent into a circular arc and back out through two equal clothoid spirals. Each spiral turns through the spiral angle, shifts the circular arc inward by p, and moves its start back along the tangent by k. Those two offsets give the total tangent from the PI back to the TS; the stations follow by adding the spiral, the remaining arc, and the second spiral. The operation shares one calculation path with `survey.curves.spiral`, which also gives the deflection table and the plan.

## Equations

- θs = Ls / (2R)
- X and Y of the spiral's end from the clothoid series in θs
- p = Y − R(1 − cos θs), k = X − R·sin θs
- Ts = (R + p)·tan(Δ/2) + k
- Arc = R·(Δ − 2θs)
- TS = PI − Ts, SC = TS + Ls, CS = SC + arc, ST = CS + Ls

## Symbols and units

Ls spiral length, R radius of the circular arc, Δ total deflection at the PI, θs spiral angle. Lengths share one unit; stations are written 12+34.56 in feet and 1+234.567 in meters.

## Domain

Positive spiral length and radius; Δ greater than 2θs and under 180°, so a circular arc remains; θs no more than 30°, where the series holds its accuracy.

## Approximations

The clothoid coordinates come from a truncated series, accurate to better than 0.001 of the unit inside the 30° limit. Both spirals are equal.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: add-survey-suite specification
- sourceEdition: 2026
- sourceLocator: survey/alignment-curves, scenario "Spiral-curve-spiral stations"; the arc length is checked by hand, not taken from a published table
- independent: no
- inputs: Ls 200 ft, R 1,000 ft, Δ 40°, PI 50+00
- outputs: θs = 0.1 rad, arc = 1,000 × (0.698132 − 0.2) = 498.132 ft, total length 898.132 ft
- tolerance: 1e-6 ft
- verifiedBy: golden vector v002 and the `named_curve_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_survey.py` `spiral_curve_spiral`: 18 random spiraled curves in all three length units, with the clothoid integrated numerically by Simpson's rule over 2,000 intervals instead of the core's series; total tangent, arc, and length agree within 1e-6.
- `core/vectors/survey.curves.spiral-curve-spiral.jsonl`: those vectors, two hand-set cases, and three refusal cases.
- The integration suite requires the same stations, total tangent, and arc as `survey.curves.spiral`.

## Invariants

- ST − TS = 2·Ls + arc.
- SC − TS = ST − CS = Ls.
- The arc shrinks to zero as Δ approaches 2θs.
