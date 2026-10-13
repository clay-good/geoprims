<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Fuel weight (`aviation.loading.fuel-weight`)

## Method

Fuel weight is volume times density, and volume is weight divided by density. The density is the one the user enters, or else the nominal value for the chosen fuel from the FAA Weight and Balance Handbook: 6.0 lb per US gallon for 100LL avgas and 6.7 lb per US gallon for Jet A. A nominal density adds the NOMINAL_VALUE_USED warning. The arithmetic runs in base units (kilograms, cubic meters) and is converted back for display.

## Equations

- Weight from volume: W = V × ρ.
- Volume from weight: V = W ÷ ρ.
- ρ = the entered density, else 6.0 lb/galUS (100LL) or 6.7 lb/galUS (Jet A).

## Symbols and units

V in US gallons by default (any volume unit accepted); W in pounds by default (any mass unit); ρ in lb/galUS by default (any density unit). The output names the density used and whether it was "nominal" or "entered".

## Domain

Exactly one of volume or weight; a fuel type or a density; density > 0; volume ≥ 0. Otherwise INVALID_INPUT.

## Approximations

The nominal densities are textbook values. Real fuel density varies with temperature and batch by a few percent, so a measured density should be entered when loading is close to a limit. Tank capacity and unusable fuel are not modeled.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Pilot's Handbook of Aeronautical Knowledge
- sourceEdition: FAA-H-8083-25C (2023)
- sourceLocator: Chapter 10, Figure 10-6, sample loading problem, usable fuel row
- independent: yes
- inputs: volume 40 US gal, fuel 100LL
- outputs: weight 240 lb at 6 lb/gal, labeled nominal by the tool
- tolerance: exact to the printed values
- verifiedBy: golden vector v006 and `core/crates/gp-aviation/tests/slice2.rs` `fuel_weight_nominal`
- verifiedOn: 2026-10-13

Figure 10-5 independently gives a second row: 75 gal of avgas weighs 450 lb. Both printed products are golden vectors.

## Differential tests

- `tools/vectors/gen_fuel_weight.py`: a separate Python evaluation of volume × density and its inverse across both nominal fuels, an entered-density override, zero fuel, and three refused inputs; the arithmetic is trivial, so this checks unit handling and the density table, not a separate method
- `core/vectors/aviation.loading.fuel-weight.jsonl`: those cases and the two printed FAA products, 20 vectors in all, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/slice2.rs` `fuel_weight_invariants`: doubling volume doubles weight; equivalent gallons and liters give the same weight and density; converting 240 lb back gives 40 gal; an entered density overrides the selected nominal fuel and removes the nominal-value warning
