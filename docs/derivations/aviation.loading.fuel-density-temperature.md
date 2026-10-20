<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Fuel density at a temperature (`aviation.loading.fuel-density-temperature`)

## Method

Fuel expands as it warms, so a gallon of warm fuel weighs less than a gallon of cold. The tool starts from the density at 15 °C, either the one on the fuel slip or a typical value for the fuel type (Chevron's Aviation Fuels Technical Review, Figure 2.1), and multiplies it by a volume correction factor (VCF) for the fuel's temperature. By default the factor comes from the formula behind the API MPMS 11.1 (1980) petroleum tables, which Measurement Canada's bulletin V-18 names for aviation fuel: Table 54B for Jet A and avgas, and Table 54A for wide-cut Jet B. The linear method uses one expansion coefficient instead: Chevron's typical 0.00099 per °C for kerosine-type fuel, or one the user enters. Both factors are reported, so their difference shows. With a volume, the tool also gives that volume's weight and its volume at 15 °C.

## Equations

- ΔT = T − 15 °C.
- Tables: VCF = exp(−α ΔT (1 + 0.8 α ΔT)), with α = K0 / ρ15² + K1 / ρ15.
- Table 54B zones (kg/m³ at 15 °C): below 770.5, K0 = 346.4228 and K1 = 0.4388 (gasolines); 770.5 to 787.5, α = −0.00336312 + 2680.3206 / ρ15² (transition); 787.5 to 838.5, K0 = 594.5418 and K1 = 0 (jet fuels); 838.5 and above, K0 = 186.9696 and K1 = 0.4862 (fuel oils).
- Table 54A (wide-cut): K0 = 613.9723, K1 = 0.
- Linear: VCF = 1 / (1 + β ΔT), β = 0.00099 per °C unless entered.
- Density: ρ(T) = ρ15 × VCF. Weight: W = V × ρ(T). Volume at 15 °C: V15 = V × VCF.

## Symbols and units

T is the fuel temperature in °C; ρ15 is the density at 15 °C in kg/m³; α and β are per °C; VCF is a pure number, the volume at 15 °C divided by the volume at T. Volumes default to US gallons and weights to pounds; any unit of the same kind is accepted.

## Domain

T from −50 to 60 °C and ρ15 from 653 to 1,075 kg/m³ (the API tables' product range), else OUT_OF_DOMAIN. The linear method with avgas, wide-cut, or a density outside the jet fuel zone needs an entered coefficient, else INVALID_INPUT. A temperature outside −40 to 40 °C adds MODEL_EXTRAPOLATED, since Measurement Canada's tables, which check the formula, stop there.

## Approximations

The zone edges are where α runs on continuously from one zone to the next, and the tool computes in double precision without the API standard's intermediate rounding (API 2540). So a factor can differ from a printed table by one unit in the fourth decimal where the exact value falls on a rounding half-step. Twelve of the 2,393 cells checked do. The API constants come from ABB's published function manual, because the standard itself is sold. They are confirmed by reproducing Measurement Canada's printed tables. The typical densities are vendor values. Jet A batches range from 775 to 840 kg/m³ at 15 °C (ASTM D1655, as Chevron prints it), so a typical density adds NOMINAL_VALUE_USED. The sentence calls the result nominal because the batch decides it.

## Worked example

- sourcePublisher: Measurement Canada
- sourceTitle: Volume correction factors to 15 °C for Jet A, Jet A-1, jet kerosene, turbine fuel
- sourceEdition: issued January 2016
- sourceLocator: the −20 °C and 30 °C rows of the table for 800 kg/m³ (Table 54B)
- independent: yes
- inputs: Jet A, density 800 kg/m³ at 15 °C, temperatures −20 °C and 30 °C
- outputs: VCF 1.0322 and 0.9860; Chevron's linear factor at −20 °C is 1.0359, 0.0037 higher
- tolerance: 0.00006 (half the table's last digit plus the standard's rounding)
- verifiedBy: golden vectors v001 and v006 and `core/crates/gp-aviation/tests/fuel_density.rs` `fuel_density_temperature_invariants`
- verifiedOn: 2026-10-20

Chevron's own example, 1.000 gal of jet fuel at 0 °C becoming 1.040 gal at 40 °C, holds on the linear method within 0.001 gal. The tables' formula gives 1.038 gal for the same warming.

## Differential tests

- Every printed cell of Measurement Canada's Jet A (800 kg/m³), Jet B (760 kg/m³, Table 54A), and aviation gasoline (710 kg/m³) tables, −40 to 40 °C every 0.1 °C, was checked against an independent Python evaluation on 2026-10-20. All 2,393 agree within 0.00006, and all but twelve agree exactly after rounding to four decimals.
- `tools/vectors/gen_fuel_density.py`: 13 printed table cells (among them the half-step cases), the typical densities through the formula, the transition and fuel-oil zones, a weight, Chevron's linear method, and four refusals; 26 vectors in `core/vectors/aviation.loading.fuel-density-temperature.jsonl`.

## Invariants

- `core/crates/gp-aviation/tests/fuel_density.rs` `fuel_density_temperature_invariants`: the seven Jet A table points; VCF 1 at 15 °C; density falls as temperature rises from −50 to 60 °C; Chevron's 1.000 to 1.040 gal; the two methods part by 0.003 to 0.004 at −20 °C; no step at the Table 54B zone edges; weight equals volume × density; the refusals.
- `core/crates/gp-aviation/tests/fuel_density.rs` `a_corrected_density_goes_to_fuel_weight_with_its_temperature`: the fuel weight tool takes 0.822 kg/L at −10 °C and names the temperature in its sentence.
