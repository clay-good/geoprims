<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Length converter (`units.length.convert`)

## Method

Every unit here is defined exactly in terms of the meter, so a conversion is one multiplication by a ratio of two exact numbers. The value is read with its unit, multiplied by the source unit's definition, divided by the target's, and rounded once. Rounding once is the whole of the method: converting feet to meters and meters to inches as two separate roundings loses a bit that a single ratio keeps.

The two feet are kept as separate units rather than as one with a footnote. The international foot is exactly 0.3048 m by the 1959 agreement; the US survey foot is exactly 1200/3937 m, which is 0.30480060960121924 m. They differ by two parts per million — nothing over a room, a tenth of a millimeter over a meter, and about two feet across a state plane zone.

## Equations

- Conversion: y = x · (definition of the source unit in meters) / (definition of the target unit in meters).
- International foot: 0.3048 m exactly; inch = foot/12; yard = 3 ft; mile = 5280 ft.
- US survey foot: 1200/3937 m exactly.
- Nautical mile: 1852 m exactly.
- The metric prefixes are exact powers of ten.

## Symbols and units

x is the value in the source unit and y the value in the target. The units are m, km, cm, mm, µm, Mm, ft (international), ftUS (US survey), in, yd, mi (international statute) and NM (international nautical).

## Domain

Any finite value, positive, negative or zero, in any of the twelve units, to any other. There is no range limit: the arithmetic is one multiplication and a very large or very small value converts as readily as an ordinary one.

## Approximations

None in the definitions, which are exact integers or exact ratios. The only inexactness is the single rounding to binary64 at the end, which is half an ulp — about one part in 10¹⁶, or a nanometre over the diameter of the Earth.

## Worked example

- sourcePublisher: National Institute of Standards and Technology
- sourceTitle: Guide for the Use of the International System of Units (SI), NIST SP 811; and NIST Handbook 44
- sourceEdition: SP 811, 2008 edition, Appendix B.8
- sourceLocator: foot = 0.3048 m exactly; US survey foot = 1200/3937 m; nautical mile = 1852 m
- independent: yes
- inputs: 22 conversions, including 5,280 ft to m, 1,000,000 ftUS to m against 1,000,000 ft to m, and every ordered pair of the twelve units
- outputs: the converted value in each case
- tolerance: 5e-16 relative, which is at most two roundings of an exact ratio
- verifiedBy: golden vectors v001 to v022, run by the core on every build
- verifiedOn: 2026-09-23

The expected values do not come from another converter. `tools/vectors/gen_units.py` restates each unit from the published definition and does the arithmetic in exact rational fractions — Python's `Fraction`, not floating point — rounding to binary64 only at the very end. So the reference has no rounding error of its own to hide behind, and the tolerance is set by what one correctly rounded ratio costs rather than by what two libraries happen to agree on.

The survey foot is the case worth stating. A million US survey feet is 304,800.6096 m and a million international feet is 304,800 m: 61 cm apart. Both are in the vectors, next to each other, because a converter that quietly treated them as the same unit would pass every other test in this file.

A cross-check against `pint` during this work disagreed on nothing here, but did disagree elsewhere in the family — on the acre and the inch of mercury — and in both cases the catalog was right and the library used the other convention. That is why the reference is the published definition rather than a second library.

## Differential tests

- `tools/vectors/gen_units.py`: all 22 vectors, from exact definitions in rational arithmetic
- `core/crates/gp-units/tests/units.rs`: the catalog lint, the examples, and the vectors, run on every build
- `core/vectors/units.length.convert.jsonl`: 22 vectors, nine hand-picked and thirteen from a sweep of every ordered pair

## Invariants

- `core/crates/gp-units/tests/units.rs` `length_invariants`: converting to a unit and back returns the value to the last bit or two, for every ordered pair; converting to the same unit changes nothing at all; zero converts to zero in every pair and a negative value keeps its sign; the conversion is linear — twice the input gives twice the output, and the sum of two values converts to the sum of their conversions; a metre is more feet than it is yards and more yards than it is miles, so the units order the way they should; and a million US survey feet and a million international feet differ by 0.6096 m, the two-parts-per-million gap that a converter conflating them would lose
