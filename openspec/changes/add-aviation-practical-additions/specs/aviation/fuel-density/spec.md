## Purpose

Weighs fuel at the temperature it is loaded, so a hot or cold ramp does not put the weight-and-balance off by a few percent.

## ADDED Requirements

### Requirement: Density and weight by temperature
The tool SHALL compute density at a temperature as ρ(T) = ρ₁₅ × VCF and the weight of a given volume. By default the volume correction factor SHALL be the API MPMS 11.1 (1980) tables' formula, VCF = exp(−α ΔT (1 + 0.8 α ΔT)) with α = K0/ρ₁₅² + K1/ρ₁₅, from Table 54B by density zone, or Table 54A for wide-cut fuel, as Measurement Canada's bulletin V-18 assigns them. A linear option SHALL give VCF = 1 / (1 + β (T − 15 °C)) with β = 0.00099 /°C for kerosine-type fuels (Chevron review, vendor guidance) or a user value, and both factors SHALL be shown. Presets are labeled typical (kerosine-type jet fuel 0.810 g/mL, wide-cut 0.762, avgas 0.715 at 15 °C, from the Chevron review), or the user enters a density at 15 °C. It SHALL state the ASTM D1655 density range (775–840 kg/m³ at 15 °C) as the batch-variation note and SHALL label the result nominal.

Spec note (2026-10-20): the first draft used the linear model for every answer, but Measurement Canada's tables are Table 54B, and the linear model misses them by 0.0037 at −20 °C and 0.0074 at −40 °C. Its two scenarios could not both pass, so the tables' formula is the default and the linear model is the option that reproduces Chevron's example.

#### Scenario: Chevron example
- **WHEN** 1.000 US gal of jet fuel at 0 °C is warmed to 40 °C with the linear method
- **THEN** its volume is 1.040 gal (±0.001 gal) at the same mass

#### Scenario: Volume correction check
- **WHEN** Jet A with 800 kg/m³ at 15 °C is evaluated at −20 °C and at 30 °C
- **THEN** the volume correction factors are 1.0322 and 0.9860 within 0.00006 of Measurement Canada's table, and the linear factor is shown beside them

#### Scenario: Temperature out of range
- **WHEN** the temperature is below −50 °C or above 60 °C
- **THEN** the result is `OUT_OF_DOMAIN` naming the validated range

### Requirement: Feeds the fuel weight tool
A density at temperature SHALL be sendable to `aviation.loading.fuel-weight` and the weight-and-balance tool, replacing the nominal preset with the temperature-corrected value and noting it.

#### Scenario: Send to fuel weight
- **WHEN** the reader sends 0.822 kg/L at −10 °C to the fuel-weight tool
- **THEN** that tool uses it and its sentence names the temperature
