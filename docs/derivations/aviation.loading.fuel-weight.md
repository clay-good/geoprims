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

- sourcePublisher: geoprims (spec scenario, computed from the handbook's nominal density)
- sourceTitle: add-aviation-suite fuel-and-loading spec, scenario "Fuel weight"
- sourceEdition: add-aviation-suite change; density from FAA-H-8083-1B (2016)
- sourceLocator: openspec/changes/add-aviation-suite/specs/aviation/fuel-and-loading/spec.md, scenario "Fuel weight" (40 US gal of 100LL at the nominal 6.0 lb/gal is 240 lb, labeled nominal); the 6.0 lb/gal density is from FAA-H-8083-1B chapter 2, and the 240 lb product is the spec's
- independent: no
- inputs: volume 40 US gal, fuel 100LL
- outputs: weight 240 lb, density 6.0 lb/gal, basis nominal
- tolerance: 1e-9 lb
- verifiedBy: golden vector v001 and `core/crates/gp-aviation/tests/slice2.rs` `fuel_weight_nominal`
- verifiedOn: 2026-10-09

## Differential tests

- `tools/vectors/gen_aviation.py`: the same products written in Python at four volumes (100LL, Jet A, and an entered density) and one weight back to volume (within 1e-12 relative). The arithmetic is trivial, so this checks unit handling and the density table, not a separate method
- `core/vectors/aviation.loading.fuel-weight.jsonl`: those five vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/slice2.rs` `fuel_weight_nominal`: no test checks an invariant of this tool directly. This test runs the spec scenario and asserts 240 lb (within 1e-9), the "nominal" basis, the NOMINAL_VALUE_USED warning, and that the summary says "nominal"
