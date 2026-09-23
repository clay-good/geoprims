<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Battery charge converter (`units.charge.convert`)

## Method

The charge conversion is exact. Every unit here is defined in terms of the coulomb, and an ampere hour is an ampere for an hour — 3,600 C exactly, because the ampere and the second are both SI. A milliampere hour is a thousandth of that, 3.6 C exactly. So a conversion is one multiplication by a ratio of two exact numbers, rounded once.

The energy output is a different kind of claim and is optional for that reason. A charge is not an energy: the energy a pack holds is its charge times the voltage it delivers, and that voltage falls as the pack discharges. When a nominal voltage is given, this reports charge × voltage — 5,000 mAh at 22.2 V is 111 Wh — which is the figure a shipping rule or a spec sheet quotes. It is a nominal, not a measurement: the real delivered energy depends on the discharge curve, the current and the temperature, and is typically a few percent either side.

## Equations

- Charge: y = x · (source unit in C) / (target unit in C).
- Ampere hour: 1 A · 3,600 s = 3,600 C exactly. Milliampere hour: 3.6 C exactly.
- Energy, when a voltage is given: E = Q · V, with Q in ampere hours and V in volts giving watt hours.

## Symbols and units

x is the charge in the source unit and y in the target; Q is the charge, V the nominal pack voltage and E the energy. The charge units are C, mAh and Ah; voltage is in V, mV or kV; energy comes back in Wh.

## Domain

Any finite charge, and any finite voltage. A negative charge is accepted arithmetically but is not what a battery capacity means.

## Approximations

The charge conversion has none: the definitions are exact and the only inexactness is the single final rounding. The energy is exact arithmetic on an inexact input — it is as good as the nominal voltage, which is a convention for a pack (3.7 V a cell for lithium polymer, so 22.2 V for a six-cell pack) rather than a measured quantity. The result is reported to four significant figures for that reason, and the tool says so in its accuracy line.

## Worked example

- sourcePublisher: National Institute of Standards and Technology
- sourceTitle: Guide for the Use of the International System of Units (SI), NIST SP 811
- sourceEdition: 2008 edition, Section 4 and Appendix B.8
- sourceLocator: the coulomb is the ampere second; the ampere hour is 3.6 kC
- independent: yes
- inputs: 22 conversions, including 5,000 mAh to Ah, 1 Ah to C, 2,200 mAh to C, and every ordered pair of the three units
- outputs: the converted value in each case
- tolerance: 5e-16 relative, at most two roundings of an exact ratio
- verifiedBy: golden vectors v001 to v022, run by the core on every build
- verifiedOn: 2026-09-23

The expected values come from `tools/vectors/gen_units.py`, which restates each unit from its published definition and converts in exact rational arithmetic — Python `Fraction`, not floating point — rounding to binary64 once at the end. The reference therefore has no rounding error of its own, and the tolerance is set by what one correctly rounded ratio costs rather than by what two libraries happen to agree on.

The watt hour the energy output reports is the same watt hour the energy converter defines, 3,600 J, so a pack energy computed here and an energy converted there are the same quantity and not two conventions that happen to share a name.

## Differential tests

- `tools/vectors/gen_units.py`: all 22 vectors, from the exact definitions in rational arithmetic
- `core/crates/gp-units/tests/units.rs`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/units.charge.convert.jsonl`: 22 vectors, the hand-picked cases and every ordered pair of the units, in both directions

## Invariants

- `core/crates/gp-units/tests/units.rs` `charge_invariants`: every ordered pair round trips to the last bit, and a unit converted to itself is untouched; the conversion is linear, so zero stays zero, a sign is kept and twice the input is twice the output; an ampere hour is exactly 3,600 C and a milliampere hour exactly 3.6 C, to the last bit, and a thousand milliampere hours are exactly one ampere hour; the energy output appears only when a voltage is given and not otherwise; and it is charge times voltage — a 5,000 mAh pack at 22.2 V comes to 111 Wh, and doubling either the charge or the voltage doubles it, which a formula that had picked up a stray factor would not do
