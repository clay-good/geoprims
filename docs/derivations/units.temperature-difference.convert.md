<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Temperature difference converter (`units.temperature-difference.convert`)

## Method

A difference between two temperatures carries the size of the scale's degree and none of its offset — the offset cancels when one reading is subtracted from another. So this conversion is a single multiplication, and the 273.15 and 459.67 that dominate the temperature converter do not appear at all.

That is why it is a separate tool. The same number means two different things depending on whether it is a reading or a change, and no amount of care inside one converter can tell which was meant. Splitting them makes the question explicit at the point where the user already knows the answer.

## Equations

- y = x · (size of the source degree) / (size of the target degree).
- Degree sizes relative to the kelvin: K = 1, °C = 1, °F = 5/9.
- So a change of 1 °C is a change of 1 K and of 1.8 °F, and nothing else enters.

## Symbols and units

x is the difference in the source scale's degrees and y in the target's. The scales are K, degC and degF. A negative value is an ordinary input: temperatures fall.

## Domain

Any finite difference, of either sign. There is no lower bound, because a difference has no absolute zero to run into — unlike a temperature, where below 0 K is refused.

## Approximations

None. The only ratio involved is 5/9, which is exact, and the single rounding to binary64 at the end.

## Worked example

- sourcePublisher: National Institute of Standards and Technology
- sourceTitle: Guide for the Use of the International System of Units (SI), NIST SP 811
- sourceEdition: 2008 edition, Appendix B.8
- sourceLocator: the degree Fahrenheit as 5/9 K, and the degree Celsius as equal to the kelvin
- independent: yes
- inputs: 22 conversions, including an ISA deviation of +18 °F to kelvins, 1 °C to °F, and every ordered pair of the three scales at several magnitudes and both signs
- outputs: the converted difference in each case
- tolerance: 5e-16 relative
- verifiedBy: golden vectors v001 to v022, run by the core on every build
- verifiedOn: 2026-09-23

`tools/vectors/gen_units.py` carries the degree sizes as exact rationals and converts in rational arithmetic, rounding once. An ISA deviation of +18 °F is exactly 10 K, which is the kind of round answer that makes a dropped offset obvious: as a temperature, 18 °F is 265.372… K.

The vectors and the temperature converter's vectors are deliberately readable side by side. 0 °C is 273.15 K there and 0 K here; 10 °C is 283.15 K there and 10 K here. The pair of files is itself the argument for two tools.

## Differential tests

- `tools/vectors/gen_units.py`: all 22 vectors, from the exact degree sizes in rational arithmetic
- `core/crates/gp-units/tests/units.rs`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/units.temperature-difference.convert.jsonl`: 22 vectors, five hand-picked and the rest from a sweep

## Invariants

- `core/crates/gp-units/tests/units.rs` `temperature_difference_invariants`: every ordered pair round trips to the last bit; a scale converted to itself is untouched; kelvins and Celsius degrees are the same size, so those two directions change the number by nothing at all — asserted as exact equality, not as a tolerance; the conversion is linear, so twice the input is twice the output and zero converts to zero, which is precisely what the temperature converter must not do; a negative difference stays negative and keeps its magnitude; 1 °C is exactly 1.8 °F and 18 °F is exactly 10 K; and the same number put through this tool and through the temperature converter differs by the scale offset, which is the confusion these two tools are split to prevent
