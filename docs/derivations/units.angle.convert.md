<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Angle converter (`units.angle.convert`)

## Method

Every angle unit here is a fixed fraction of a full turn, so a conversion is one multiplication by a ratio. Most of those ratios are exact whole-number fractions: a degree is a 360th of a turn, a gon a 400th, an arcsecond a 1,296,000th. Radians are the exception, being a turn divided by 2π, so conversions touching radians or milliradians carry an irrational factor and are computed in floating point rather than exactly.

The three military mils are separate units, not one unit with a note. A mil is meant to be about a milliradian — an angle subtending one metre at a thousand — but 2π thousandths of a turn is not a convenient number to fire artillery with, so different armies rounded it differently. NATO divides the turn into 6,400, the Warsaw Pact into 6,000, and Sweden into 6,300. A number of mils is therefore a different angle depending on whose manual it came from, and nothing in the number itself says which.

## Equations

- Conversion: y = x · (source unit as a fraction of a turn) / (target unit as a fraction of a turn).
- Exact fractions of a turn: degree 1/360, gon 1/400, arcminute 1/21,600, arcsecond 1/1,296,000, milliarcsecond 1/1,296,000,000, NATO mil 1/6,400, Warsaw mil 1/6,000, Swedish mil 1/6,300.
- Radian: 1/(2π) of a turn; milliradian a thousandth of that.
- So 1 NATO mil is exactly 0.9375 Warsaw mils, and 1,600 NATO mils is exactly 90°.

## Symbols and units

x is the angle in the source unit, y in the target. The units are deg, rad, gon, arcmin, arcsec, mas, mil-nato, mil-warsaw, mil-sweden, mrad and turn.

## Domain

Any finite angle, of either sign and any size. Angles are not wrapped: 400° stays 400° and does not become 40°, because this converts the size of an angle and has no idea whether it is a bearing, a rotation or an accumulated total.

## Approximations

The exact units convert through exact ratios with one rounding. Radians and milliradians involve π, so those conversions use the double-precision value of π and carry a tolerance of 2e-15 rather than 5e-16 — about a nanoarcsecond on a right angle, which is far below anything measurable.

## Worked example

- sourcePublisher: National Institute of Standards and Technology
- sourceTitle: Guide for the Use of the International System of Units (SI), NIST SP 811
- sourceEdition: 2008 edition, Appendix B.8
- sourceLocator: degree = (π/180) rad; minute = (1/60)°; second = (1/60)′; gon (grade) = (π/200) rad
- independent: yes
- inputs: 110 conversions, including 1,600 NATO mils to degrees, 100 gon to degrees, 180° to radians, and every ordered pair of the eleven units
- outputs: the converted angle in each case
- tolerance: 5e-16 relative for the exact units, 2e-15 where π is involved
- verifiedBy: golden vectors v001 to v110, run by the core on every build
- verifiedOn: 2026-09-23

`tools/vectors/gen_units.py` holds each unit as an exact `Fraction` of a degree and converts in rational arithmetic, rounding once at the end. The units involving π are kept in a separate table and computed with `math.pi`, and they are the only ones that get the looser tolerance — which is why the two tolerances exist rather than one loose one covering both.

The mils are the reason this tool is worth a note. All three convert a full turn to 360°: 6,400 NATO mils, 6,000 Warsaw mils and 6,300 Swedish mils. A converter that carried only one definition and accepted all three names would give the right answer for one army and be wrong by 6% for another, with nothing in the output to show it.

## Differential tests

- `tools/vectors/gen_units.py`, `gen_units_gaps.py` and `gen_units_pairs.py`: all 110 vectors, from exact fractions of a turn in rational arithmetic
- `core/crates/gp-units/tests/units.rs`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/units.angle.convert.jsonl`: 110 vectors, the hand-picked cases and every ordered pair of the units, in both directions

## Invariants

- `core/crates/gp-units/tests/units.rs` `angle_invariants`: every ordered pair round trips to the last bit or two, and a unit converted to itself is untouched; the conversion is linear, so zero stays zero, a sign is kept, and twice the input is twice the output; a full turn is 360°, 400 gon, 2π rad, 6,400 NATO mils, 6,000 Warsaw mils and 6,300 Swedish mils, each asserted separately, so a mil resolving to the wrong definition fails here and nowhere else; the three mils are related to each other exactly — 1 NATO mil is 0.9375 Warsaw mils — which a shared definition would make 1; the sexagesimal chain holds, 1° being 60 arcminutes, 3,600 arcseconds and 3,600,000 milliarcseconds; and angles are not wrapped, so 400° converts as 400° and not as 40°
