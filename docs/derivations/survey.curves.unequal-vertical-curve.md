<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Unequal-tangent vertical curve (`survey.curves.unequal-vertical-curve`)

## Method

The tool joins two parabolas at the station directly under the PVI. The first begins tangent to the entering grade at the PVC; the second ends tangent to the leaving grade at the PVT. A length-weighted grade makes their slopes equal where they meet. The tool evaluates the relevant parabola within the curve and extends the appropriate tangent before the PVC or after the PVT.

## Equations

- Common grade under the PVI: `gc = (L1 g1 + L2 g2) / (L1 + L2)`
- First rate of grade change: `r1 = (gc - g1) / L1`
- Second rate of grade change: `r2 = (g2 - gc) / L2`
- PVC: `station = PVI - L1`, `elevation = E_PVI - g1 L1`
- PVT: `station = PVI + L2`, `elevation = E_PVI + g2 L2`
- Curve elevation under the PVI: `E_CVC = E_PVI + L1 L2 (g2 - g1) / (2(L1 + L2))`
- First parabola, distance `x` from the PVC: `y = E_PVC + g1 x + r1 x² / 2`
- Second parabola, distance `x` from the PVI: `y = E_CVC + gc x + r2 x² / 2`
- A turning point on either segment is at `x = -g / r` when that distance lies on the segment.

## Symbols and units

`g1`, `g2`, and `gc` are decimal grades in the equations and percentages at the interface. `L1` is the horizontal length from PVC to PVI and `L2` is the length from PVI to PVT. `r1` and `r2` are grade change per unit of horizontal length. `E_PVI`, `E_PVC`, `E_CVC`, and `E_PVT` are elevations. Lengths and elevations may be entered in one compatible unit; international feet and US survey feet cannot be mixed.

## Domain

Both tangent lengths must be positive. Grades may be positive, negative, zero, or equal. Equal grades describe a straight profile and therefore have no interior high or low point. A requested station may lie on either parabola or on a tangent extension outside the curve. Station text must use a form such as `10+00` in feet or `4+300.000` in meters.

## Approximations

The parabolas are the geometric definition, so calculation is exact apart from floating-point rounding. This is alignment geometry, not a design check. It does not select lengths, check sight distance, drainage, comfort, or overhead clearance, or decide whether an unequal-tangent curve is appropriate.

## Worked example

- sourcePublisher: Pearson Education
- sourceTitle: Solutions to Problems, Elementary Surveying: An Introduction to Geomatics
- sourceEdition: 15th edition, 2015
- sourceLocator: Problem 25.15, unequal-tangent vertical curve
- independent: yes
- inputs: g1 = +2.50%, g2 = -1.25%, PVI station 60+00 at 3,310.00 ft, L1 = 600 ft, L2 = 400 ft
- outputs: PVC 54+00 at 3,295.000 ft; curve under the PVI at 3,305.500 ft; PVT 64+00 at 3,305.000 ft; elevation at 61+00 is 3,306.219 ft
- tolerance: 0.001 ft on the source's three-decimal elevations and exact stations
- verifiedBy: golden vector v008 and `unequal_vertical_curve_invariants`, run by the core on every build
- verifiedOn: 2026-10-13

The unrounded second-parabola equation gives 3,306.21875 ft at 61+00, which rounds to the source's 3,306.219 ft.

## Differential tests

- `tools/vectors/gen_survey.py` independently solves the common grade, both parabolas, tangent extensions, and turning points. Its 20 vectors cover crest, sag, monotone, straight, endpoint-turning, metric, US-unit, and invalid-input cases.
- `core/vectors/survey.curves.unequal-vertical-curve.jsonl` runs those vectors through the shared core on every build.

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `unequal_vertical_curve_invariants` checks that equal tangent lengths reproduce the stable equal-tangent tool, the two parabolas meet with the same elevation and grade under the PVI, and the published problem 25.15 result is reproduced.
