## Purpose

Weighs fuel at the temperature it is loaded, so a hot or cold ramp does not put the weight-and-balance off by a few percent.

## ADDED Requirements

### Requirement: Density and weight by temperature
The tool SHALL compute density at a temperature as ρ(T) = ρ₁₅ / (1 + β (T − 15 °C)) and the weight of a given volume, with presets labeled typical (kerosine-type jet fuel 0.810 g/mL, wide-cut 0.762, avgas 0.715 at 15 °C, from the Chevron review, vendor guidance) or a user density at 15 °C, and β = 0.00099 /°C for kerosine-type fuels (same source) or a user value. It SHALL state the ASTM D1655 density range (775–840 kg/m³ at 15 °C) as the batch-variation note and SHALL label the result nominal.

#### Scenario: Chevron example
- **WHEN** 1.000 US gal of jet fuel at 0 °C is warmed to 40 °C
- **THEN** its volume is 1.040 gal (±0.001 gal) at the same mass

#### Scenario: Volume correction check
- **WHEN** Jet A with 800 kg/m³ at 15 °C is evaluated at −20 °C and at 30 °C
- **THEN** the volume correction factors are 1.0322 and 0.9860 within 0.0015 of Measurement Canada's table, and the difference is shown

#### Scenario: Temperature out of range
- **WHEN** the temperature is below −50 °C or above 60 °C
- **THEN** the result is `OUT_OF_DOMAIN` naming the validated range

### Requirement: Feeds the fuel weight tool
A density at temperature SHALL be sendable to `aviation.loading.fuel-weight` and the weight-and-balance tool, replacing the nominal preset with the temperature-corrected value and noting it.

#### Scenario: Send to fuel weight
- **WHEN** the reader sends 0.822 kg/L at −10 °C to the fuel-weight tool
- **THEN** that tool uses it and its sentence names the temperature
