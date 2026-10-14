<!-- Derivation note (trust/correctness-program layer A). -->
# Curve stationing (`survey.curves.curve-stationing`)

## Method

Stationing runs along the alignment. The curve begins one tangent length back from the PI, and ends one curve length ahead of where it began, measured along the arc. The operation solves the circular curve from any two elements and reports the PC and PT stations. It shares one calculation path with `survey.curves.circular-curve`.

## Equations

- T = R·tan(Δ/2), L = R·Δ with Δ in radians
- PC station = PI station − T
- PT station = PC station + L

## Symbols and units

R radius, Δ deflection angle at the PI, T tangent, L curve length. Lengths share one unit. Stations are written 12+34.56 in feet and 1+234.567 in meters.

## Domain

The PI station and exactly two independent curve elements are required. Elements must be positive and the deflection between 0° and 360°. Two elements that fit two curves return the first with AMBIGUOUS_INPUT, as the circular curve tool does.

## Approximations

None beyond rounding stations to 0.01 ft or 0.001 m.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: add-survey-suite specification
- sourceEdition: 2026
- sourceLocator: survey/alignment-curves, requirement "Circular curves" (PC and PT stations from the PI station); checked by hand arithmetic, not taken from a published table
- independent: no
- inputs: PI 62+17.08, R 1,432.40 ft, Δ 24°
- outputs: T 304.47 ft, L 600.00 ft, PC 59+12.61, PT 65+12.62
- tolerance: 0.01 ft
- verifiedBy: golden vector v001 and the `named_curve_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_survey.py` `curve_stationing`: 18 random curves in all three length units, defined in turn by the deflection, the tangent, and the curve length, by a separate Python evaluation.
- `core/vectors/survey.curves.curve-stationing.jsonl`: those vectors, two hand-checked cases, and three refusal cases.
- The integration suite requires the same stations and elements as `survey.curves.circular-curve`.

## Invariants

- PT station − PC station = curve length.
- PI station − PC station = tangent.
- The same curve entered by different pairs of elements has the same stations.
