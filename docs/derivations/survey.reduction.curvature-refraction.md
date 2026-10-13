<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Curvature and refraction correction (`survey.reduction.curvature-refraction`)

## Method

The tool models Earth as a sphere and computes how far the curved surface falls below a straight horizontal sight over the entered distance. Atmospheric refraction bends the sight toward Earth and cancels the fraction `k` of that curvature. The tool reports the combined correction, the curvature and refraction parts separately, and the equivalent coefficient in meters per square kilometer.

## Equations

- Curvature: `C = D² / (2R)`
- Refraction part: `F = −kD² / (2R)`
- Combined correction: `h = C + F = (1 − k)D² / (2R)`
- Coefficient: `q = (1 − k) × 10⁶ / (2R)` meters per square kilometer

The negative sign on `F` states that ordinary terrestrial refraction reduces the curvature correction.

## Symbols and units

`D` is horizontal sight distance, `R` is Earth radius, `k` is the dimensionless terrestrial-refraction coefficient, `C` is curvature alone, `F` is the signed refraction part, and `h` is their combined correction. The calculation converts lengths to meters. Length results are returned in the distance input's unit; `q` is always meters per square kilometer.

## Domain

Distance must be finite and nonnegative. Earth radius must be finite and positive; its default is 6,371,000 m. The refraction coefficient is from 0 through 1 and defaults to 0.13. At `k = 0`, there is no refraction. At `k = 1`, this simplified model makes refraction cancel curvature exactly.

## Approximations

The equation uses a sphere of one radius and one coefficient along the whole sight. Real refraction changes with air-density gradients, temperature, ground conditions, height above ground, and time of day. The default 6,371,000 m is GRS 80's mean radius rounded to the kilometer. Reciprocal observations and balanced backsights and foresights reduce curvature and refraction uncertainty more reliably than assuming a coefficient. The tool does not convert a slope distance to horizontal distance; reduce the sight first when that distinction matters.

## Worked example

- sourcePublisher: U.S. Geological Survey
- sourceTitle: Procedures and Best Practices for Trigonometric Leveling in the U.S. Geological Survey
- sourceEdition: Techniques and Methods 11-D3 (2020)
- sourceLocator: Equation 10 and table 4, 500 ft row
- independent: yes
- inputs: 500 ft distance and `k = 1/7`, the refraction ratio stated immediately above equation 10
- outputs: 0.0060 ft curvature, 0.0009 ft refraction magnitude, and 0.0051 ft combined correction
- tolerance: 0.00005 ft, half the table's 0.0001 ft display unit
- verifiedBy: golden vector v007 and `core/crates/gp-survey/tests/survey.rs`
- verifiedOn: 2026-10-13

With the tool's stated 6,371,000 m radius, it returns 0.005980 ft curvature, −0.000854 ft refraction, and 0.005126 ft combined. Each rounds to the USGS table's published value.

## Differential tests

- `tools/vectors/gen_survey.py`: 25 vectors independently evaluate the USGS table distances, the spec's 1 km coefficients, typical and endpoint refraction coefficients, three Earth radii, equivalent length units, zero and long sights, and invalid distance and radius inputs

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `curvature_refraction_matches_the_usgs_table`: reproduces all three values in the USGS 500 ft row at printed precision
- `core/crates/gp-survey/tests/survey.rs` `curvature_refraction_grows_with_distance_squared`: doubling distance multiplies every correction by four
- `core/crates/gp-survey/tests/survey.rs` `curvature_refraction_parts_recombine`: curvature plus the signed refraction part equals the combined correction
- `core/crates/gp-survey/tests/survey.rs` `curvature_refraction_coefficient_endpoints_are_consistent`: `k = 0`, `0.5`, and `1` follow the linear coefficient model
- `core/crates/gp-survey/tests/survey.rs` `curvature_refraction_is_unit_invariant_and_names_invalid_fields`: equivalent radii and distances agree across units, and bad inputs identify their field
