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

- sourcePublisher: geoprims
- sourceTitle: add-survey-suite specification
- sourceEdition: 2026
- sourceLocator: survey/alignment-curves, requirement "Circular curves" (D = 5,729.578 / R for a 100 ft arc); the constant is the one route-surveying texts print, checked here by arithmetic rather than against a published table
- independent: no
- inputs: radius 5,729.578 ft
- outputs: degree of curve 1.0000000° by the arc definition
- tolerance: 1e-7°
- verifiedBy: golden vector v002 and the `named_curve_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_survey.py` `degree_of_curve`: 17 random cases from a radius in each length unit and from each degree definition, by a separate Python evaluation.
- `core/vectors/survey.curves.degree-of-curve.jsonl`: those vectors, three hand-checked cases, and five refusal cases.
- The integration suite requires the same degrees as `survey.curves.circular-curve` for the same radius.

## Invariants

- Radius to arc degree and back returns the radius.
- Radius to chord degree and back returns the radius.
- The chord-definition degree is larger than the arc-definition degree for the same radius.
