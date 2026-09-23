<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Fuel volume and weight (`units.fuel.convert`)

## Method

Fuel weight is one multiplication: mass = volume × density. The arithmetic is trivial and the density is not, so almost everything here is about which density is used and how honest the answer is about it.

Give a measured density and that is what is used. Give a fuel type instead and a nominal planning density is used — 6 lb per US gallon for avgas 100LL, 6.7 for Jet A — and the result carries a `NOMINAL_VALUE_USED` warning saying so. Give neither and the tool refuses rather than guessing, with a hint naming both nominal figures. Give both and it refuses too, because there is no sensible rule for which one wins.

The multiplication is done in the density's own units, so 50 US gallons at 6 lb per US gallon is exactly 300 lb rather than 299.99999999999994. That matters because a weight-and-balance sheet is read by a person who would notice.

## Equations

- mass = volume × density; volume = mass / density.
- Pound: 0.45359237 kg exactly. US liquid gallon: 3.785411784 L exactly.
- avgas 100LL, nominal: 6 lb/US gal. Jet A at 15 °C, nominal: 6.7 lb/US gal.

## Symbols and units

Volume in galUS, L, m³ or ft³; mass in lb or kg; density in lb/galUS, g/cm³ or kg/m³. Exactly one of volume and mass is given and the other comes back, along with the density that was used.

## Domain

Any finite positive volume or mass, with any finite positive density. Give a volume or a mass, not both. Give a fuel type or a density, not both, and not neither.

## Approximations

The unit definitions are exact. The nominal densities are not: a fuel's density changes with temperature and batch by roughly ±3%, so a nominal figure is a planning number and the tool says so in the result rather than in a footnote. Jet A's 6.7 lb/gal is quoted at 15 °C; on a hot day the same gallons weigh less.

## Worked example

- sourcePublisher: Federal Aviation Administration; National Institute of Standards and Technology
- sourceTitle: Pilot's Handbook of Aeronautical Knowledge, FAA-H-8083-25C; Guide for the Use of the International System of Units (SI), NIST SP 811
- sourceEdition: FAA-H-8083-25C (2023); SP 811, 2008 edition, Appendix B.8
- sourceLocator: FAA-H-8083-25C Chapter 10, Weight and Balance — avgas 6 lb/gal, jet fuel 6.7 lb/gal; pound = 0.45359237 kg and US gallon = 3.785 411 784 L exactly
- independent: yes
- inputs: 20 cases, including 50 gal of avgas to pounds, 300 lb of avgas back to gallons, 200 L of Jet A, 0.5 m³ at 0.8 g/cm³, and a volume with no density at all
- outputs: the mass or the volume, and the density used
- tolerance: 5e-16 relative
- verifiedBy: golden vectors v001 to v020, run by the core on every build
- verifiedOn: 2026-09-23

The expected values come from `tools/vectors/gen_units.py` and `gen_units_special.py`. The second one deliberately takes a different route than the core: it converts the volume to cubic metres and the density to kilograms per cubic metre, both from the exact published definitions in rational arithmetic, multiplies there and converts the answer to pounds. The core instead works in the density's own units. The two agree to a bit or two across all twenty cases, which is what the tolerance allows — and the exactness the core's route buys is checked separately, by an invariant rather than by a vector.

## Differential tests

- `tools/vectors/gen_units.py` and `gen_units_special.py`: all 20 vectors, the second computing mass in SI rather than in the density's units
- `core/crates/gp-units/tests/units.rs`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/units.fuel.convert.jsonl`: 20 vectors, both directions, all four volume units and three ways of giving a density

## Invariants

- `core/crates/gp-units/tests/units.rs` `fuel_invariants`: 50 US gallons of avgas is exactly 300.0 lb and not a float a hair under it, which is what working in the density's own units buys; the two directions invert each other, so a volume turned into a mass and back is the volume it started as; the answer is linear in both arguments, twice the volume and twice the density each doubling the mass; the nominal densities are the published 6.0 and 6.7 lb/gal; a fuel type brings a `NOMINAL_VALUE_USED` warning and a measured density does not; and both refusals hold — no density at all, and a fuel type and a density together
