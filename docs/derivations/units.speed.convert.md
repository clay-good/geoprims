<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Speed converter (`units.speed.convert`)

## Method

Every unit here is defined exactly in terms of the SI unit, so a conversion is one multiplication by a ratio of two exact numbers: the value times the source unit's definition, divided by the target's, rounded once. Rounding once is the method. Converting through an intermediate unit in two steps rounds twice and loses a bit that a single ratio keeps.

The knot is the unit that makes this domain worth its own tool. It is a nautical mile an hour, and the international nautical mile is exactly 1,852 m, so a knot is exactly 1.852 km/h and exactly 463/900 m/s. Every other unit here follows from a length and a time that are themselves exact.

## Equations

- Conversion: y = x · (source unit in m/s) / (target unit in m/s).
- Knot: 1852/3600 m/s exactly, so 1 kt = 1.852 km/h exactly.
- Mile per hour: 1609.344/3600 m/s = 0.44704 m/s exactly.
- Foot per second: 0.3048 m/s exactly.
- Millimetre and metre per year: the SI year here is the Julian year of 365.25 days.

## Symbols and units

x is the speed in the source unit and y in the target. The units are m/s, km/h, kt, mph, ft/s, mm/yr and m/yr.

## Domain

Any finite speed of either sign. A negative speed is accepted: a closing rate, a component along an axis and a subsidence rate are all signed quantities.

## Approximations

None in the definitions. The single rounding to binary64 is the only inexactness, at half an ulp.

## Worked example

- sourcePublisher: National Institute of Standards and Technology
- sourceTitle: Guide for the Use of the International System of Units (SI), NIST SP 811; and NIST Handbook 44
- sourceEdition: SP 811, 2008 edition, Appendix B.8
- sourceLocator: knot = 1852/3600 m/s; mile per hour = 0.44704 m/s exactly
- independent: yes
- inputs: 47 conversions, including 100 kt to mph, 88 ft/s to mph, 1 kt to m/s, and every ordered pair of the seven units
- outputs: the converted value in each case
- tolerance: 5e-16 relative, at most two roundings of an exact ratio
- verifiedBy: golden vectors v001 to v047, run by the core on every build
- verifiedOn: 2026-09-23

The expected values come from `tools/vectors/gen_units.py`, which restates each unit from its published definition and converts in exact rational arithmetic — Python `Fraction`, not floating point — rounding to binary64 once at the end. The reference therefore has no rounding error of its own, and the tolerance is set by what one correctly rounded ratio costs rather than by what two libraries happen to agree on.

The geodetic rates are in the same table as the aviation ones on purpose. Plate motion is quoted in millimetres a year and a descent in feet a minute, and a converter that treated them as separate domains would need two answers for what is one ratio.

## Differential tests

- `tools/vectors/gen_units.py`, `gen_units_gaps.py` and `gen_units_pairs.py`: all 47 vectors, from the exact definitions in rational arithmetic
- `core/crates/gp-units/tests/units.rs`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/units.speed.convert.jsonl`: 47 vectors, the hand-picked cases and every ordered pair of the units, in both directions

## Invariants

- `core/crates/gp-units/tests/units.rs` `speed_invariants`: every ordered pair round trips to the last bit or two, and a unit converted to itself is untouched; the conversion is linear, so zero stays zero, a sign is kept and twice the input is twice the output; a knot is exactly 1.852 km/h and a mile per hour exactly 0.44704 m/s, both to the last bit, since those are the two definitions everything else in the table leans on; a metre a year is exactly 1,000 millimetres a year; and the units order by size, a given speed being more km/h than ft/s, more ft/s than mph, more mph than knots and more knots than metres per second -- the knot being the largest of the four, so a speed is fewer knots than mph and not more, which is the way round it is easy to write down backwards
