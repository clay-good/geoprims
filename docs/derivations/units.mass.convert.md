<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Mass converter (`units.mass.convert`)

## Method

Every unit here is defined exactly in terms of the SI unit, so a conversion is one multiplication by a ratio of two exact numbers: the value times the source unit's definition, divided by the target's, rounded once. Rounding once is the method. Converting through an intermediate unit in two steps rounds twice and loses a bit that a single ratio keeps.

The pound is the unit everything imperial here rests on, and since 1959 it has been defined rather than measured: exactly 0.45359237 kg. The ounce is a sixteenth of it, exactly 28.349523125 g, which is a terminating decimal because 0.45359237/16 happens to be one.

## Equations

- Conversion: y = x · (source unit in kg) / (target unit in kg).
- Pound: 0.45359237 kg exactly (international avoirdupois, 1959).
- Ounce: pound/16 = 0.028349523125 kg exactly.
- Tonne: 1,000 kg exactly. Gram: 1/1,000 kg.

## Symbols and units

x is the mass in the source unit and y in the target. The units are kg, g, lb, oz and t (metric tonne).

## Domain

Any finite mass. Negative values are accepted because a mass difference — a fuel burn, a payload change — is a signed quantity even though a mass is not.

## Approximations

None. The definitions are exact decimals and the only inexactness is the single final rounding.

## Worked example

- sourcePublisher: National Institute of Standards and Technology
- sourceTitle: Guide for the Use of the International System of Units (SI), NIST SP 811; and NIST Handbook 44
- sourceEdition: SP 811, 2008 edition, Appendix B.8
- sourceLocator: pound (avoirdupois) = 0.453 592 37 kg exactly; ounce = 1/16 lb
- independent: yes
- inputs: 24 conversions, including 2,550 lb to kg, 16 oz to lb, 1 t to lb, and every ordered pair of the five units
- outputs: the converted value in each case
- tolerance: 5e-16 relative, at most two roundings of an exact ratio
- verifiedBy: golden vectors v001 to v024, run by the core on every build
- verifiedOn: 2026-09-23

The expected values come from `tools/vectors/gen_units.py`, which restates each unit from its published definition and converts in exact rational arithmetic — Python `Fraction`, not floating point — rounding to binary64 once at the end. The reference therefore has no rounding error of its own, and the tolerance is set by what one correctly rounded ratio costs rather than by what two libraries happen to agree on.

The pound of mass is the one the pressure converter uses to build psi, and the two tools agree by construction rather than by coincidence: psi is this pound times standard gravity over the square inch. The invariant in the pressure note checks that product rather than a copied decimal, which is what keeps the two in step.

## Differential tests

- `tools/vectors/gen_units.py`, `gen_units_gaps.py` and `gen_units_pairs.py`: all 24 vectors, from the exact definitions in rational arithmetic
- `core/crates/gp-units/tests/units.rs`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/units.mass.convert.jsonl`: 24 vectors, the hand-picked cases and every ordered pair of the units, in both directions

## Invariants

- `core/crates/gp-units/tests/units.rs` `mass_invariants`: every ordered pair round trips to the last bit, and a unit converted to itself is untouched; the conversion is linear, so zero stays zero, a sign is kept and twice the input is twice the output; a pound is exactly 0.45359237 kg and an ounce exactly 28.349523125 g, to the last bit; sixteen ounces are exactly one pound, which a rounded ounce would miss; and a tonne is exactly 1,000 kg and not a short or long ton, both of which are close enough to pass a loose check
