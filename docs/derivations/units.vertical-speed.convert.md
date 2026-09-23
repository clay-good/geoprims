<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Vertical speed converter (`units.vertical-speed.convert`)

## Method

Every unit here is defined exactly in terms of metres per second, so a conversion is one multiplication by a ratio of two exact numbers: the value times the source unit's definition, divided by the target's, rounded once. Rounding once is the method. Converting through an intermediate unit in two steps rounds twice and loses a bit that a single ratio keeps.

There are only two constants behind the four units: the international foot of exactly 0.3048 m, and the minute of exactly 60 s. A foot per minute is therefore exactly 0.00508 m/s, a terminating decimal, so the conversion an altimeter or a vertical-speed indicator needs is exact rather than approximated.

Vertical speed is kept as its own quantity rather than folded into speed, because the units differ: a rate of climb is quoted in feet per minute and a ground speed in knots, and offering knots as a climb rate would invite a reading nobody means.

## Equations

- Conversion: y = x · (source unit in m/s) / (target unit in m/s).
- Foot per minute: 0.3048/60 = 0.00508 m/s exactly.
- Foot per second: 0.3048 m/s exactly. Metre per minute: 1/60 m/s.

## Symbols and units

x is the rate in the source unit and y in the target. The units are m/s, ft/min, m/min and ft/s.

## Domain

Any finite rate. Negative values are not only accepted but expected: a descent is a negative climb, and the sign is the difference between the two.

## Approximations

None. Both defining constants are exact and the only inexactness is the single final rounding.

## Worked example

- sourcePublisher: National Institute of Standards and Technology; International Civil Aviation Organization
- sourceTitle: Guide for the Use of the International System of Units (SI), NIST SP 811; Annex 5 to the Convention on International Civil Aviation
- sourceEdition: SP 811, 2008 edition, Appendix B.8; Annex 5, 5th edition (2010)
- sourceLocator: foot = 0.3048 m exactly; Annex 5 Chapter 3 and Table 3-3 list the foot per minute as the non-SI alternative permitted for rate of climb or descent
- independent: yes
- inputs: 22 conversions, including 500 ft/min to m/s, −700 ft/min to m/s, 1,000 ft/min to m/min, and every ordered pair of the four units
- outputs: the converted value in each case
- tolerance: 5e-16 relative, at most two roundings of an exact ratio
- verifiedBy: golden vectors v001 to v022, run by the core on every build
- verifiedOn: 2026-09-23

The expected values come from `tools/vectors/gen_units.py`, which restates each unit from its published definition and converts in exact rational arithmetic — Python `Fraction`, not floating point — rounding to binary64 once at the end. The reference therefore has no rounding error of its own, and the tolerance is set by what one correctly rounded ratio costs rather than by what two libraries happen to agree on.

The foot here is the same international foot the length, speed, energy and pressure converters use, so a climb rate and a distance stay consistent with each other by construction rather than by coincidence.

## Differential tests

- `tools/vectors/gen_units.py`: all 22 vectors, from the exact definitions in rational arithmetic
- `core/crates/gp-units/tests/units.rs`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/units.vertical-speed.convert.jsonl`: 22 vectors, the hand-picked cases and every ordered pair of the units, in both directions

## Invariants

- `core/crates/gp-units/tests/units.rs` `vertical_speed_invariants`: every ordered pair round trips to the last bit, and a unit converted to itself is untouched; the conversion is linear, so zero stays zero, a sign is kept and twice the input is twice the output; a foot per minute is exactly 0.00508 m/s and a foot per second exactly 0.3048 m/s, to the last bit; sixty feet per minute are exactly one foot per second, which a rounded ft/min would miss; and a descent stays a descent — −500 ft/min converts to a negative rate in every unit, since a sign quietly dropped here would turn a descent into a climb
