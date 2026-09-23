<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Pressure converter (`units.pressure.convert`)

## Method

Each unit is defined in pascals and a conversion is one multiplication by a ratio, but the units get their definitions three different ways, and knowing which matters for reading the last digits.

Some are exact by fiat: the bar is 100,000 Pa, the standard atmosphere is 101,325 Pa, and the metric prefixes are powers of ten. Some are derived from other exact definitions: the pound per square inch is the pound-force — the international pound times standard gravity — divided by the square inch, which makes it exactly 6894.757293168361… Pa without anyone having chosen that number. And the mercury units are conventional: the inch of mercury is *defined* as exactly 3386.389 Pa and the millimetre as exactly 133.322387415 Pa, rather than computed from the density of mercury at some temperature.

## Equations

- Conversion: y = x · (source unit in pascals) / (target unit in pascals).
- Exact: bar = 10⁵ Pa, atm = 101,325 Pa, hPa = mbar = 100 Pa, kPa = 1,000 Pa.
- Derived: psi = (lb × g₀) / in² = (0.45359237 kg × 9.80665 m/s²) / (0.0254 m)² .
- Conventional: inHg = 3386.389 Pa, mmHg = 133.322387415 Pa.

## Symbols and units

x is the pressure in the source unit and y in the target. The units are Pa, hPa, kPa, mbar, bar, inHg, mmHg, psi and atm. All are absolute in the sense that they are sizes of pressure; nothing here distinguishes absolute from gauge.

## Domain

Any finite pressure of either sign. A negative pressure is accepted because a differential or a gauge reading below ambient is a real quantity; this tool converts the number it is given and does not decide what it was measured against.

## Approximations

None in the definitions, which are exact integers or exact rational products. The single rounding to binary64 at the end is the only inexactness. What is conventional rather than exact is the mercury: a real manometer, corrected for the density of its own mercury at its own temperature, will differ from these definitions in about the seventh digit, and that is a fact about mercury rather than about the arithmetic.

## Worked example

- sourcePublisher: National Institute of Standards and Technology
- sourceTitle: Guide for the Use of the International System of Units (SI), NIST SP 811
- sourceEdition: 2008 edition, Appendix B.8
- sourceLocator: inch of mercury (conventional) = 3.386 389 E+03 Pa; millimetre of mercury (conventional) = 1.333 224 E+02 Pa; standard atmosphere = 1.013 25 E+05 Pa
- independent: yes
- inputs: 73 conversions, including 29.92 inHg to hPa, 1013.25 hPa to inHg, 1 psi to Pa, 760 mmHg to hPa, and every ordered pair of the nine units
- outputs: the converted pressure in each case
- tolerance: 5e-16 relative
- verifiedBy: golden vectors v001 to v073, run by the core on every build
- verifiedOn: 2026-09-23

`tools/vectors/gen_units.py` carries each unit as an exact `Fraction` of a pascal — including psi, built up from the pound and standard gravity as fractions rather than as a decimal — and converts in rational arithmetic, rounding once.

A cross-check against `pint` during this work disagreed here by one part in ten million, and the catalog was right: pint derives `inch_Hg` as 3386.38864 Pa from the density of mercury, while NIST SP 811 gives the conventional inch of mercury as exactly 3386.389 Pa. Both are defensible; only one is the published constant, and it is the one a chart or an altimeter means.

One detail the vectors make visible: 29.92 inHg is 1013.2076 hPa, not 1013.25. The familiar altimeter setting is a rounded number, and the standard atmosphere is 29.9212524 inHg. A converter that quietly treated 29.92 inHg and 1013.25 hPa as the same pressure would be out by 4 Pa — about a foot of altitude.

## Differential tests

- `tools/vectors/gen_units.py`, `gen_units_gaps.py` and `gen_units_pairs.py`: all 73 vectors, from exact definitions in rational arithmetic
- `core/crates/gp-units/tests/units.rs`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/units.pressure.convert.jsonl`: 73 vectors, the hand-picked cases and every ordered pair of the units, in both directions

## Invariants

- `core/crates/gp-units/tests/units.rs` `pressure_invariants`: every ordered pair round trips to the last bit or two, and a unit converted to itself is untouched; the conversion is linear, so zero stays zero, a sign is kept and twice the input is twice the output; the exact definitions hold to the last bit — a bar is 100,000 Pa, an atmosphere 101,325 Pa, a millibar and a hectopascal are the same pressure; the conventional mercury units are exactly their published constants, 3386.389 Pa and 133.322387415 Pa, so a converter deriving them from mercury's density fails here; psi is the pound-force over the square inch, checked against that product computed in the test rather than against a copied decimal; and the standard atmosphere is 29.9212524 inHg while 29.92 inHg is 1013.2076 hPa, the two being four pascals apart
