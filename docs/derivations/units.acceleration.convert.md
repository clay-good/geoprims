<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Acceleration converter (`units.acceleration.convert`)

## Method

Every unit here is defined exactly in terms of metres per second squared, so a conversion is one multiplication by a ratio of two exact numbers: the value times the source unit's definition, divided by the target's, rounded once. Rounding once is the method. Converting through an intermediate unit in two steps rounds twice and loses a bit that a single ratio keeps.

Two constants do the work. The international foot is exactly 0.3048 m, so a foot per second squared is exactly 0.3048 m/s². Standard gravity, written g₀, is exactly 9.80665 m/s² — a defined constant, not a measurement, which is the point of it: it is the conventional value the CGPM fixed so that a load factor or a g-rating means the same number everywhere. The local acceleration of gravity is not this number; it varies with latitude and height by a few parts in a thousand, and a g here is the defined one.

## Equations

- Conversion: y = x · (source unit in m/s²) / (target unit in m/s²).
- Standard gravity: g₀ = 9.80665 m/s² exactly.
- Foot per second squared: 0.3048 m/s² exactly.
- So 1 g₀ = 32.174048556430446… ft/s² exactly, which is 9.80665/0.3048 and not the rounded 32.174 or 32.2 a textbook prints.

## Symbols and units

x is the acceleration in the source unit and y in the target. The units are m/s², g0 (standard gravity) and ft/s².

## Domain

Any finite acceleration. Negative values are accepted and meaningful: a deceleration, or a negative load factor.

## Approximations

None. Both constants are exact decimals and the only inexactness is the single final rounding.

## Worked example

- sourcePublisher: National Institute of Standards and Technology
- sourceTitle: Guide for the Use of the International System of Units (SI), NIST SP 811
- sourceEdition: 2008 edition, Appendix B.8
- sourceLocator: standard acceleration of free fall, g_n = 9.806 65 m/s² exactly; foot = 0.3048 m exactly
- independent: yes
- inputs: 22 conversions, including 2 g to m/s², 1 g to ft/s², 9.80665 m/s² to g, −1.5 g to m/s², and every ordered pair of the three units
- outputs: the converted value in each case
- tolerance: 5e-16 relative, at most two roundings of an exact ratio
- verifiedBy: golden vectors v001 to v022, run by the core on every build
- verifiedOn: 2026-09-23

The expected values come from `tools/vectors/gen_units.py`, which restates each unit from its published definition and converts in exact rational arithmetic — Python `Fraction`, not floating point — rounding to binary64 once at the end. The reference therefore has no rounding error of its own, and the tolerance is set by what one correctly rounded ratio costs rather than by what two libraries happen to agree on.

This is the same standard gravity that turns the pound of mass into the pound-force behind psi, the foot pound-force and the horsepower. The four tools take it from one definition rather than four decimals, which is what keeps them in step.

## Differential tests

- `tools/vectors/gen_units.py`: all 22 vectors, from the exact definitions in rational arithmetic
- `core/crates/gp-units/tests/units.rs`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/units.acceleration.convert.jsonl`: 22 vectors, the hand-picked cases and every ordered pair of the units, in both directions

## Invariants

- `core/crates/gp-units/tests/units.rs` `acceleration_invariants`: every ordered pair round trips to the last bit, and a unit converted to itself is untouched; the conversion is linear, so zero stays zero, a sign is kept and twice the input is twice the output; one g is exactly 9.80665 m/s² and one foot per second squared exactly 0.3048 m/s², to the last bit; and one g in ft/s² is the correctly rounded 32.17404855643045, which is neither the textbook 32.174 nor what dividing the two doubles gives. That last one is the point: 9.80665 and 0.3048 are each already rounded as binary64, so their quotient rounds twice and lands one bit low at ...044, while combining the exact definitions into a single ratio and rounding once gives ...045. The test pins that bit rather than hiding it behind a tolerance, because rounding once is the method the whole converter rests on
