<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Energy converter (`units.energy.convert`)

## Method

Every unit here is defined exactly in terms of the joule, so a conversion is one multiplication by a ratio of two exact numbers: the value times the source unit's definition, divided by the target's, rounded once. Rounding once is the method. Converting through an intermediate unit in two steps rounds twice and loses a bit that a single ratio keeps.

Four of the six units are decimal multiples of the joule and need no explanation. The other two are worth stating. A watt hour is a watt for an hour, so exactly 3,600 J, and a kilowatt hour exactly 3,600,000 J — these are definitions, not measurements, because the second and the watt are both SI. The foot pound-force is built from three exact numbers: the international foot of 0.3048 m, the avoirdupois pound of 0.45359237 kg, and standard gravity of 9.80665 m/s². Their product is 1.3558179483314 J exactly, a terminating decimal, which is why it can be carried here without approximation.

## Equations

- Conversion: y = x · (source unit in J) / (target unit in J).
- Watt hour: 3,600 J exactly. Kilowatt hour: 3,600,000 J exactly.
- Pound-force: 0.45359237 kg · 9.80665 m/s² = 4.4482216152605 N exactly.
- Foot pound-force: 0.3048 m · 4.4482216152605 N = 1.3558179483314 J exactly.

## Symbols and units

x is the energy in the source unit and y in the target. The units are J, kJ, MJ, Wh, kWh and ft·lbf (foot pound-force).

## Domain

Any finite energy. Negative values are accepted because an energy change — a battery discharged, work done against a load — is a signed quantity even though a stored energy is not.

## Approximations

None. Every definition is an exact decimal and the only inexactness is the single final rounding.

## Worked example

- sourcePublisher: National Institute of Standards and Technology
- sourceTitle: Guide for the Use of the International System of Units (SI), NIST SP 811
- sourceEdition: 2008 edition, Appendix B.8 and B.9
- sourceLocator: foot pound-force = 1.355 817 948 331 4 E+00 J (exact); watt hour = 3.6 E+03 J (exact)
- independent: yes
- inputs: 34 conversions, including 99 Wh to kJ, 1 kWh to MJ, 1,000 ft·lbf to kWh, and every ordered pair of the six units
- outputs: the converted value in each case
- tolerance: 5e-16 relative, at most two roundings of an exact ratio
- verifiedBy: golden vectors v001 to v034, run by the core on every build
- verifiedOn: 2026-09-23

The expected values come from `tools/vectors/gen_units.py`, which restates each unit from its published definition and converts in exact rational arithmetic — Python `Fraction`, not floating point — rounding to binary64 once at the end. The reference therefore has no rounding error of its own, and the tolerance is set by what one correctly rounded ratio costs rather than by what two libraries happen to agree on.

The foot pound-force reproduces NIST's published digits exactly, to all fourteen of them, because it is built from the same three defined constants rather than copied from a table. The same pound and the same standard gravity build psi in the pressure converter and the pound of mass in the mass converter, so the three tools agree by construction. The invariant test checks the product rather than a transcribed decimal, which is what keeps them in step when one of them is edited.

## Differential tests

- `tools/vectors/gen_units.py`, `gen_units_gaps.py` and `gen_units_pairs.py`: all 34 vectors, from the exact definitions in rational arithmetic
- `core/crates/gp-units/tests/units.rs`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/units.energy.convert.jsonl`: 34 vectors, the hand-picked cases and every ordered pair of the units, in both directions

## Invariants

- `core/crates/gp-units/tests/units.rs` `energy_invariants`: every ordered pair round trips to the last bit, and a unit converted to itself is untouched; the conversion is linear, so zero stays zero, a sign is kept and twice the input is twice the output; a watt hour is exactly 3,600 J and a kilowatt hour exactly 3,600,000 J, to the last bit; a kilowatt hour is exactly 3.6 MJ, which a rounded watt hour would miss; and the foot pound-force equals the foot times the pound times standard gravity, computed from those three constants rather than compared against a copied decimal
