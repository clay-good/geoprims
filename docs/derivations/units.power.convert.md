<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Power converter (`units.power.convert`)

## Method

Every unit here is defined exactly in terms of the watt, so a conversion is one multiplication by a ratio of two exact numbers: the value times the source unit's definition, divided by the target's, rounded once. Rounding once is the method. Converting through an intermediate unit in two steps rounds twice and loses a bit that a single ratio keeps.

The watt and the kilowatt need no explanation. The horsepower does, because the word names several different units. This one is the mechanical, or imperial, horsepower: 550 foot pounds-force per second, which is Watt's own definition and the one a US or UK engine rating means. Built from the international foot, the avoirdupois pound and standard gravity, it is 745.69987158227 W exactly. The metric horsepower (PS, 735.49875 W) and the boiler horsepower (about 9,810 W) are different units and are not offered here, because a converter that guessed between them would be wrong by 1.4% or by a factor of thirteen with no way for the reader to tell.

## Equations

- Conversion: y = x · (source unit in W) / (target unit in W).
- Pound-force: 0.45359237 kg · 9.80665 m/s² = 4.4482216152605 N exactly.
- Horsepower: 550 · 0.3048 m · 4.4482216152605 N/s = 745.69987158227 W exactly.
- Kilowatt: 1,000 W exactly.

## Symbols and units

x is the power in the source unit and y in the target. The units are W, kW and hp (mechanical horsepower).

## Domain

Any finite power. Negative values are accepted because a power flow has a direction — a battery charging rather than discharging, a motor regenerating rather than driving.

## Approximations

None. Every definition is an exact decimal and the only inexactness is the single final rounding.

## Worked example

- sourcePublisher: National Institute of Standards and Technology
- sourceTitle: Guide for the Use of the International System of Units (SI), NIST SP 811
- sourceEdition: 2008 edition, Appendix B.8 and B.9
- sourceLocator: horsepower (550 ft · lbf/s) = 7.456 998 715 822 7 E+02 W (exact)
- independent: yes
- inputs: 22 conversions, including 180 hp to kW, 1 hp to W, 100 kW to hp, and every ordered pair of the three units
- outputs: the converted value in each case
- tolerance: 5e-16 relative, at most two roundings of an exact ratio
- verifiedBy: golden vectors v001 to v022, run by the core on every build
- verifiedOn: 2026-09-23

The expected values come from `tools/vectors/gen_units.py`, which restates each unit from its published definition and converts in exact rational arithmetic — Python `Fraction`, not floating point — rounding to binary64 once at the end. The reference therefore has no rounding error of its own, and the tolerance is set by what one correctly rounded ratio costs rather than by what two libraries happen to agree on.

The horsepower reproduces NIST's published digits exactly, to all thirteen of them, because it is built from the same three defined constants rather than copied from a table. The same pound and the same standard gravity build the foot pound-force in the energy converter, psi in the pressure converter and the pound of mass in the mass converter, so the four agree by construction. The invariant test checks the product rather than a transcribed decimal, which is what keeps them in step when one of them is edited.

## Differential tests

- `tools/vectors/gen_units.py`: all 22 vectors, from the exact definitions in rational arithmetic
- `core/crates/gp-units/tests/units.rs`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/units.power.convert.jsonl`: 22 vectors, the hand-picked cases and every ordered pair of the units, in both directions

## Invariants

- `core/crates/gp-units/tests/units.rs` `power_invariants`: every ordered pair round trips to the last bit, and a unit converted to itself is untouched; the conversion is linear, so zero stays zero, a sign is kept and twice the input is twice the output; a kilowatt is exactly 1,000 W; the horsepower equals 550 times the foot times the pound times standard gravity, computed from those constants rather than compared against a copied decimal; and it is the mechanical horsepower and not the metric one — the two are 1.4% apart, close enough that a loose check would pass either, so the test pins the gap as well as the value
