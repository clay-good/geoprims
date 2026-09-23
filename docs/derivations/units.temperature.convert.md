<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Temperature converter (`units.temperature.convert`)

## Method

A temperature scale has two things: how big its degree is, and where it puts zero. Converting between scales means undoing both. The value is carried to kelvins by adding the source scale's offset and multiplying by its degree size, then out again by the reverse for the target. Kelvin is the pivot because its zero is the physical one and it needs no offset of its own.

This is an affine conversion, not a linear one, and that is the whole difference between this tool and the temperature-difference converter beside it. Doubling the input does not double the output here: 10 °C is 50 °F, but 20 °C is 68 °F.

## Equations

- To kelvins: K = (x + offset) × f, with (f, offset) = (1, 0) for K, (1, 273.15) for °C, and (5/9, 459.67) for °F.
- From kelvins: x = K / f − offset.
- Between two scales in one step: y = (x + offset_a) × f_a / f_b − offset_b.
- Celsius and kelvin degrees are the same size, so f is 1 for both and only the 273.15 offset separates them.

## Symbols and units

x is the temperature in the source scale, y in the target. The scales are K, degC and degF. `converted` is the value in the target scale and `input` echoes the value as read.

## Domain

Any temperature at or above absolute zero: 0 K, −273.15 °C, −459.67 °F, all three of which are accepted exactly. Below that is refused rather than converted, because no scale means anything there and a negative kelvin is far more likely to be a difference that reached the wrong tool than an intention; the refusal says so.

## Approximations

None in the definitions: 273.15 and 459.67 are exact by definition of the scales, and 5/9 is an exact ratio. The single rounding to binary64 at the end is the only inexactness, and it is half an ulp.

## Worked example

- sourcePublisher: National Institute of Standards and Technology
- sourceTitle: Guide for the Use of the International System of Units (SI), NIST SP 811
- sourceEdition: 2008 edition, Appendix B.8
- sourceLocator: t/°C = T/K − 273.15; t/°F = 1.8 (t/°C) + 32
- independent: yes
- inputs: 22 conversions, including 30 °C to °F, −40 °F to °C, 0 K to °C, and every ordered pair of the three scales at several magnitudes
- outputs: the converted temperature in each case
- tolerance: 5e-16 relative, at most two roundings of an exact ratio
- verifiedBy: golden vectors v001 to v022, run by the core on every build
- verifiedOn: 2026-09-23

The reference is not another converter. `tools/vectors/gen_units.py` carries each scale as an exact rational offset and factor — `Fraction(27315, 100)` and `Fraction(5, 9)`, not floating point — and does the whole conversion in rational arithmetic, rounding to binary64 once at the end. So the expected value has no rounding error of its own, and the tolerance measures what one correctly rounded conversion costs rather than what two libraries happen to agree on.

The vectors include −40, where Celsius and Fahrenheit cross, and 0 K, where the offsets are at their most exposed. A converter that dropped an offset would land on the right answer at neither.

Two faults came out of writing this note, and both were in the code rather than in the reference. Below absolute zero was not refused at all: −300 °C came back as −26.85 K, which looks like an answer. And absolute zero *itself* was refused in every scale but kelvin — a shared range check treated a nonzero reading that lands on zero in base units as an underflow, which is right for a length and wrong for an affine scale, where −273.15 °C is exactly 0 K by definition. Both are fixed, and both are pinned by the invariants below.

## Differential tests

- `tools/vectors/gen_units.py`: all 22 vectors, from the exact scale definitions in rational arithmetic
- `core/crates/gp-units/tests/units.rs`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/units.temperature.convert.jsonl`: 22 vectors, seven hand-picked and the rest from a sweep of every ordered pair

## Invariants

- `core/crates/gp-units/tests/units.rs` `temperature_invariants`: every ordered pair round trips to the last bit or two; a scale converted to itself is untouched; the fixed points are exact — 0 °C is 273.15 K and 32 °F, 100 °C is 212 °F, −40 °C is −40 °F, and absolute zero is 0 K, −273.15 °C and −459.67 °F; below absolute zero is refused in all three scales rather than answered; the conversion is affine and not linear, so doubling the input does not double the output, which is asserted directly because it is the difference between this tool and its neighbour; and a Celsius value and the same number as a difference come out differently by exactly the offset, which is the mistake this pair of tools exists to make impossible
