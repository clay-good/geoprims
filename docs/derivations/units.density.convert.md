<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Density converter (`units.density.convert`)

## Method

A density is a mass over a volume, and every unit here is built from a mass and a volume that are themselves defined exactly. So a conversion is one multiplication by a ratio of two exact numbers: the value times the source unit's definition, divided by the target's, rounded once. Rounding once is the method. Converting through an intermediate unit in two steps rounds twice and loses a bit that a single ratio keeps.

Three constants do the work, and they are the same ones the mass and volume converters use: the avoirdupois pound of exactly 0.45359237 kg, the US liquid gallon of exactly 3.785411784 L, and the international foot of exactly 0.3048 m. A pound per US gallon is therefore 119.82642731689663 kg/m³ and a pound per cubic foot 16.018463373960138 kg/m³, both correctly rounded from an exact ratio rather than copied from a handbook.

The gallon here is the US liquid gallon. The imperial gallon is 4.54609 L, 20% larger, and a density in pounds per imperial gallon is not offered, because a figure that says only "lb/gal" would otherwise convert silently under the wrong reading.

## Equations

- Conversion: y = x · (source unit in kg/m³) / (target unit in kg/m³).
- Gram per cubic centimetre: 1,000 kg/m³ exactly — the same number as kg/L.
- Pound per US gallon: 0.45359237 kg / 0.003785411784 m³ = 119.82642731689663 kg/m³.
- Pound per cubic foot: 0.45359237 kg / 0.3048³ m³ = 16.018463373960138 kg/m³.

## Symbols and units

x is the density in the source unit and y in the target. The units are kg/m³, g/cm³ (which is also kg/L), lb/galUS and lb/ft³.

## Domain

Any finite positive density. Negative values are accepted arithmetically — the converter does not know what the number means — but a negative density is not a physical quantity, and a difference of two densities is better handled as such.

## Approximations

None. All three constants are exact decimals and the only inexactness is the single final rounding.

## Worked example

- sourcePublisher: National Institute of Standards and Technology
- sourceTitle: Guide for the Use of the International System of Units (SI), NIST SP 811; and NIST Handbook 44
- sourceEdition: SP 811, 2008 edition, Appendix B.8
- sourceLocator: pound (avoirdupois) = 0.45359237 kg exactly; gallon (US liquid) = 3.785 411 784 L exactly; foot = 0.3048 m exactly
- independent: yes
- inputs: 22 conversions, including 6.7 lb/US gal to g/cm³, 800 kg/m³ to lb/US gal, 1 g/cm³ to lb/ft³, and every ordered pair of the four units
- outputs: the converted value in each case
- tolerance: 5e-16 relative, at most two roundings of an exact ratio
- verifiedBy: golden vectors v001 to v022, run by the core on every build
- verifiedOn: 2026-09-23

The expected values come from `tools/vectors/gen_units.py`, which restates each unit from its published definition and converts in exact rational arithmetic — Python `Fraction`, not floating point — rounding to binary64 once at the end. The reference therefore has no rounding error of its own, and the tolerance is set by what one correctly rounded ratio costs rather than by what two libraries happen to agree on.

Water at 4 °C, near 1.000 g/cm³, comes out as 8.345404452019332 lb per US gallon and 62.42796057614461 lb per cubic foot — the familiar 8.345 and 62.4 of a handbook, reproduced from the definitions rather than carried as decimals. The same pound and gallon build the fuel converter next door, so a density used there and a density converted here are the same number.

## Differential tests

- `tools/vectors/gen_units.py`: all 22 vectors, from the exact definitions in rational arithmetic
- `core/crates/gp-units/tests/units.rs`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/units.density.convert.jsonl`: 22 vectors, the hand-picked cases and every ordered pair of the units, in both directions

## Invariants

- `core/crates/gp-units/tests/units.rs` `density_invariants`: every ordered pair round trips to the last bit, and a unit converted to itself is untouched; the conversion is linear, so zero stays zero, a sign is kept and twice the input is twice the output; a gram per cubic centimetre is exactly 1,000 kg/m³, which is the kg/L identity the fuel converter leans on; a pound per US gallon and a pound per cubic foot are each the pound divided by the corresponding volume, computed from the three constants rather than compared against copied decimals; and the gallon behind lb/gal is the US one — a pound per imperial gallon would be 20% smaller, a gap wide enough that the test can tell the two apart outright
