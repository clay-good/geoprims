<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Area converter (`units.area.convert`)

## Method

Every unit here is defined exactly in terms of the SI unit, so a conversion is one multiplication by a ratio of two exact numbers: the value times the source unit's definition, divided by the target's, rounded once. Rounding once is the method. Converting through an intermediate unit in two steps rounds twice and loses a bit that a single ratio keeps.

Areas are squares of lengths, so the two feet that the length converter keeps apart reappear here squared, and the four-parts-per-million gap between them doubles to eight. The acre is where it shows: the international acre is 43,560 international square feet and the US survey acre is 43,560 survey square feet, and they are different areas.

## Equations

- Conversion: y = x · (source unit in m²) / (target unit in m²).
- International acre: 43,560 × 0.3048² = 4046.8564224 m² exactly.
- US survey acre: 43,560 × (1200/3937)² = 4046.872609874252… m².
- Hectare: 10,000 m² exactly. Square mile: 640 international acres exactly.
- Square nautical mile: 1852² = 3,429,904 m² exactly.

## Symbols and units

x is the area in the source unit and y in the target. The units are m2, km2, ha, ac (international), ftUS2, acUS (US survey), ft2, mi2 and NM2.

## Domain

Any finite area. Negative values convert, since a signed area is what a cut-and-fill or a difference of parcels produces.

## Approximations

None in the definitions, which are exact rational squares. The single final rounding is the only inexactness.

## Worked example

- sourcePublisher: National Institute of Standards and Technology
- sourceTitle: Guide for the Use of the International System of Units (SI), NIST SP 811; and NIST Handbook 44
- sourceEdition: SP 811, 2008 edition, Appendix B.8
- sourceLocator: acre = 43 560 ft²; the US survey foot as 1200/3937 m
- independent: yes
- inputs: 22 conversions, including 1 acre to square feet, 40 acres to hectares, 1 square mile to acres, and every ordered pair of the units
- outputs: the converted value in each case
- tolerance: 5e-16 relative, at most two roundings of an exact ratio
- verifiedBy: golden vectors v001 to v022, run by the core on every build
- verifiedOn: 2026-09-23

The expected values come from `tools/vectors/gen_units.py`, which restates each unit from its published definition and converts in exact rational arithmetic — Python `Fraction`, not floating point — rounding to binary64 once at the end. The reference therefore has no rounding error of its own, and the tolerance is set by what one correctly rounded ratio costs rather than by what two libraries happen to agree on.

A cross-check against `pint` disagreed here by four parts per million and the catalog was right: pint's `acre` is the US survey acre, while `ac` here is the international one and `acUS` is the survey one, side by side. A library carrying only one and accepting both names would give a land record the wrong area by a square metre a section, silently.

## Differential tests

- `tools/vectors/gen_units.py`: all 22 vectors, from the exact definitions in rational arithmetic
- `core/crates/gp-units/tests/units.rs`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/units.area.convert.jsonl`: 22 vectors, the hand-picked cases and a sweep of every ordered pair

## Invariants

- `core/crates/gp-units/tests/units.rs` `area_invariants`: every ordered pair round trips to the last bit or two, and a unit converted to itself is untouched; the conversion is linear, so zero stays zero, a sign is kept and twice the input is twice the output; a hectare is exactly 10,000 m² and a square mile exactly 640 acres; a square nautical mile is exactly 3,429,904 m², the square of 1,852; and the two acres differ — a thousand international acres and a thousand survey acres are about 16 m² apart, the doubled two-parts-per-million of the two feet, which a converter conflating them would lose
