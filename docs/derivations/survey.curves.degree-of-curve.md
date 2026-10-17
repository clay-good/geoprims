<!-- Derivation note (trust/correctness-program layer A). -->
# Degree of curve converter (`survey.curves.degree-of-curve`)

## Method

Before radius became the usual way to name a curve, a curve was named by the angle it turns in 100 ft. Highways measure the 100 ft along the arc; railroads measure it along a chord. The operation converts a radius to both degrees of curve, or either degree back to the radius. It uses the same four functions as `survey.curves.circular-curve`.

## Equations

- Arc definition: D = 18,000 / (π·R), about 5,729.578 / R, with R in feet.
- Chord definition: sin(D/2) = 50 / R, so D = 2·asin(50 / R) and R = 50 / sin(D/2).

## Symbols and units

R radius; D degree of curve in degrees. A radius in meters or US survey feet is converted to international feet first, since both definitions are tied to 100 ft, and the radius is reported in the unit it was entered in (feet when a degree was entered).

## Domain

Give exactly one of the radius, the arc-definition degree, or the chord-definition degree. A radius must be positive and a degree between 0° and 180°. The chord definition has no value for a radius under 50 ft, so that output is left out there.

## Approximations

None in the arithmetic. The two definitions are different curves for the same number: a 4° curve is 1,432.39 ft by arc and 1,432.69 ft by chord.

## Worked example

- sourcePublisher: OpenExamPrep
- sourceTitle: FE Civil Guide 2026, "6.1 Transportation: Geometric Design"
- sourceEdition: last updated June 2026
- sourceLocator: Horizontal Curves section, arc-definition degree of curve R = 5,729.58 / D, worked for D = 4°
- independent: yes
- inputs: arc-definition degree of curve 4°
- outputs: radius 1,432 ft (the guide prints the whole foot; 5,729.578 / 4 = 1,432.3944878 ft)
- tolerance: the guide's radius is given to the whole foot and the core's 1,432.39 ft matches that rounding; reading the radius back returns 4.0000000° to 1e-7°
- verifiedBy: golden vector v001 and the `named_curve_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-17

## Differential tests

- `tools/vectors/gen_survey.py` `degree_of_curve`: 17 random cases from a radius in each length unit and from each degree definition, by a separate Python evaluation.
- `core/vectors/survey.curves.degree-of-curve.jsonl`: those vectors, three hand-checked cases, and five refusal cases.
- The integration suite requires the same degrees as `survey.curves.circular-curve` for the same radius.

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `degree_of_curve_invariants`: a radius gives both degrees of curve, each degree alone returns the radius it came from, and for the same radius the chord-definition degree is larger than the arc-definition degree.
