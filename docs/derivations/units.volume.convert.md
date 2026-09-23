<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Volume converter (`units.volume.convert`)

## Method

Every unit here is defined exactly in terms of the SI unit, so a conversion is one multiplication by a ratio of two exact numbers: the value times the source unit's definition, divided by the target's, rounded once. Rounding once is the method. Converting through an intermediate unit in two steps rounds twice and loses a bit that a single ratio keeps.

The two gallons are the reason this tool needs care. The US gallon is defined as 231 cubic inches, which works out to exactly 3.785411784 L; the imperial gallon is defined directly as exactly 4.54609 L. They are twenty per cent apart, they share a name in ordinary speech, and neither is called simply `gallon` here.

## Equations

- Conversion: y = x · (source unit in m³) / (target unit in m³).
- US gallon: 231 in³ = 231 × 0.0254³ = 0.003785411784 m³ exactly.
- Imperial gallon: 0.00454609 m³ exactly.
- Cubic foot: 0.3048³ = 0.028316846592 m³ exactly. Cubic yard: 27 ft³.
- Litre: 1/1,000 m³ exactly.

## Symbols and units

x is the volume in the source unit and y in the target. The units are m3, L, mL, galUS, galImp, ft3 and yd3.

## Domain

Any finite volume, of either sign — a volume difference, an uplift or a cut is signed.

## Approximations

None in the definitions, which are exact rational products. The single final rounding is the only inexactness.

## Worked example

- sourcePublisher: National Institute of Standards and Technology
- sourceTitle: Guide for the Use of the International System of Units (SI), NIST SP 811; and NIST Handbook 44
- sourceEdition: SP 811, 2008 edition, Appendix B.8
- sourceLocator: gallon (US) = 231 in³; gallon (imperial, UK) = 4.546 09 L
- independent: yes
- inputs: 45 conversions, including 50 US gallons to litres, 1 imperial gallon to US gallons, 27 cubic feet to cubic yards, and every ordered pair of the seven units
- outputs: the converted value in each case
- tolerance: 5e-16 relative, at most two roundings of an exact ratio
- verifiedBy: golden vectors v001 to v045, run by the core on every build
- verifiedOn: 2026-09-23

The expected values come from `tools/vectors/gen_units.py`, which restates each unit from its published definition and converts in exact rational arithmetic — Python `Fraction`, not floating point — rounding to binary64 once at the end. The reference therefore has no rounding error of its own, and the tolerance is set by what one correctly rounded ratio costs rather than by what two libraries happen to agree on.

One imperial gallon is 1.2009499255 US gallons. A tank filled to a number read in the wrong gallon is a fifth wrong, which on fuel is the difference between a legal reserve and none, and is exactly the kind of error that never announces itself because both numbers look plausible.

## Differential tests

- `tools/vectors/gen_units.py`, `gen_units_gaps.py` and `gen_units_pairs.py`: all 45 vectors, from the exact definitions in rational arithmetic
- `core/crates/gp-units/tests/units.rs`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/units.volume.convert.jsonl`: 45 vectors, the hand-picked cases and every ordered pair of the units, in both directions

## Invariants

- `core/crates/gp-units/tests/units.rs` `volume_invariants`: every ordered pair round trips to the last bit or two, and a unit converted to itself is untouched; the conversion is linear, so zero stays zero, a sign is kept and twice the input is twice the output; a US gallon is exactly 3.785411784 L and an imperial gallon exactly 4.54609 L, to the last bit; a cubic yard is exactly 27 cubic feet, which a rounded foot would miss; and the two gallons differ by about twenty per cent, asserted as a ratio so that a converter carrying one definition under both names fails here
