<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# RTK and PPK precision budget (`survey.gnss.rtk-budget`)

## Method

Receiver makers commonly state relative-positioning precision as a constant term plus a parts-per-million term that grows with the distance from rover to base. The tool converts the baseline and constant terms to meters, adds the baseline-dependent term directly, and reports the result in millimeters. Horizontal and vertical specifications are evaluated separately. The optional target is compared with the horizontal result at the selected confidence level.

## Equations

- One-sigma horizontal precision: `H₁ = aₕ + bₕ × 10⁻⁶ × d`
- One-sigma vertical precision: `V₁ = aᵥ + bᵥ × 10⁻⁶ × d`
- 95% horizontal precision: `H₉₅ = 2.448 × H₁`
- 95% vertical precision: `V₉₅ = 1.96 × V₁`

The horizontal factor is the rounded 95% radial factor for an isotropic two-dimensional normal distribution. The vertical factor is the two-sided 95% factor for a one-dimensional normal distribution. The two manufacturer terms are added, not combined by root sum square, because the published specification is written as `a + b ppm`.

## Symbols and units

`aₕ` and `aᵥ` are the constant horizontal and vertical terms, `bₕ` and `bᵥ` are dimensionless parts-per-million coefficients, and `d` is the rover-to-base baseline. One ppm contributes 1 mm for each 1 km of baseline. Length inputs may use any supported unit; the calculation uses meters internally and reports millimeters.

## Domain

The horizontal constant and ppm coefficient are required and nonnegative. The baseline is from 0 through 10,000 km, an implementation safety bound rather than a claim that a receiver specification remains valid that far. Vertical terms are optional but must be supplied together and be nonnegative. A target must be nonnegative. The 95% option is valid only when the entered specification is a one-sigma value.

## Approximations

The `2.448` and `1.96` confidence factors are rounded constants. The budget itself is a receiver specification, not a field uncertainty model. It does not add base-coordinate uncertainty, antenna setup error, multipath, obstructions, poor satellite geometry, atmospheric decorrelation, initialization failure, or datum and geoid errors. Product distance limits and confidence labels still govern; a specification stated as RMS, 95%, or another basis must not be scaled as though it were one sigma.

## Worked example

- sourcePublisher: Tersus GNSS Inc.
- sourceTitle: How does Tersus handle long-range single baseline RTK positioning?
- sourceEdition: Technical blog, April 6, 2023
- sourceLocator: Oscar receiver 40 km worked example
- independent: yes
- inputs: 8 mm + 1 ppm horizontal, 15 mm + 1 ppm vertical, and a 40 km baseline
- outputs: 48 mm horizontal and 55 mm vertical
- tolerance: exact to the 1 mm precision printed by the source
- verifiedBy: golden vector v006 and `core/crates/gp-survey/tests/gnss.rs`
- verifiedOn: 2026-10-13

Tersus prints the arithmetic explicitly as `8 + 1 × 40 = 48 mm` horizontally and `15 + 1 × 40 = 55 mm` vertically. The tool returns both values exactly with the confidence left at its default one-sigma basis.

## Differential tests

- `tools/vectors/gen_gnss.py`: 25 vectors independently evaluate both axes, zero and maximum baselines, ppm limits, metric and US units, one-sigma and 95% confidence, all target states, paired vertical inputs, and field-specific invalid values

## Invariants

- `core/crates/gp-survey/tests/gnss.rs` `rtk_budget_matches_the_tersus_worked_example`: reproduces both values in the manufacturer's published 40 km example
- `core/crates/gp-survey/tests/gnss.rs` `rtk_budget_one_ppm_is_one_millimeter_per_kilometer`: verifies the defining ppm relationship at 0 km, 1 km, and 40 km
- `core/crates/gp-survey/tests/gnss.rs` `rtk_budget_applies_the_documented_confidence_factors`: 95% results are exactly 2.448 and 1.96 times their one-sigma values
- `core/crates/gp-survey/tests/gnss.rs` `rtk_budget_is_unit_invariant`: equivalent centimeters, millimeters, meters, and kilometers give the same results
- `core/crates/gp-survey/tests/gnss.rs` `rtk_budget_invalid_inputs_name_the_field`: negative and incomplete terms identify the input that needs correction
