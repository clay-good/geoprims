<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Side slope and grade (`survey.earthwork.grade`)

## Method

The tool represents a grade as the signed vertical change divided by the positive horizontal run. It parses percent, per mille, degree, decimal, H:V, and V:H forms into that one ratio, then derives every other form. A ratio without H and V labels is refused until the reader chooses a convention. When any two side lengths of the right triangle are supplied, the third follows from the Pythagorean theorem.

## Equations

- Decimal grade: `g = rise / run`
- Percent grade: `100 g`
- Per mille: `1,000 g`
- Slope angle: `atan(g)`
- H:V ratio: `(1 / |g|):1`
- V:H ratio: `|g|:1`
- Slope length: `√(rise² + run²)`

## Symbols and units

`g` is a signed, dimensionless grade. Rise is the signed vertical change, run is the positive horizontal distance, and slope length is the distance along the grade. The three lengths must share one length dimension and are returned in their common input unit. Angles are returned in degrees; percent and per mille are dimensionless.

## Domain

A degree input must lie strictly between -90° and 90°. The horizontal part of a ratio and a supplied run must be positive. A slope length used to reconstruct a missing side must be longer than the supplied side. A bare ratio such as `3:1` requires an explicit H:V or V:H convention.

## Approximations

The conversions and right-triangle reconstruction are exact apart from floating-point rounding. Displayed ratios are rounded to three decimal places. The model is one straight grade; it does not model a vertical curve, a changing profile, or the geotechnical stability of a cut or fill.

## Worked example

- sourcePublisher: Federal Highway Administration
- sourceTitle: Noise Barrier Design: Terminology
- sourceEdition: Web page read 2026-10-13
- sourceLocator: Grade definition and worked roadway example
- independent: yes
- inputs: a roadway rises 20 m over a horizontal run of 400 m
- outputs: 5% grade
- tolerance: exact to the source's printed precision
- verifiedBy: golden vector v011, run by the core on every build
- verifiedOn: 2026-10-13

The source computes `20 / 400 × 100 = 5%`. Geoprims returns the same 5% and also derives the angle, labeled ratios, and 400.4997 m slope length from the same right triangle.

## Differential tests

- `tools/vectors/gen_survey.py` independently evaluates the trigonometric and right-triangle equations. The 23 vectors cover every supported notation, both bare-ratio conventions, ascending and descending grades, every pair of triangle sides, metric and US units, and invalid ambiguous, angular, distance, and triangle inputs.

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `grade_forms_and_length_invariants` checks that five notations give the same grade and that all three pairs of a 3-4-5 triangle reconstruct the same sides.
- `core/crates/gp-survey/tests/survey.rs` `ambiguous_ratio_shows_both_readings` checks that an unlabeled ratio is refused with both possible interpretations and that a labeled ratio is accepted.
