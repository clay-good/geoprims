<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Slope converter (`units.slope.convert`)

## Method

A slope has two families of description and one relationship between them. The ratio family — rise over run, percent grade, per mille — are all the same number scaled by 1, 100 or 1,000, so conversions within it are exact. The angle is not in that family: it is the arctangent of the ratio, and that is the one place a slope converter goes wrong.

The mistake worth naming is treating a percent grade as an angle. A 100% grade is 45°, not 90°, and a 10% grade is 5.71°, not 10°. The two agree to within a percent below about 10% grade, which is exactly why the error survives: on the shallow slopes most people meet it looks right.

Give the tool a value in any of the four representations and it returns all four, so the relationship is visible rather than something to look up.

## Equations

- ratio = rise / run.
- percent = 100 · ratio. Per mille = 1,000 · ratio.
- degrees = atan(ratio), in degrees; equivalently ratio = tan(degrees).

## Symbols and units

The value is a bare number and `from` says how to read it: `ratio`, `percent`, `permille` or `degrees`. All four come back — `ratio`, `percent`, `permille` and `degrees`.

## Domain

Any finite ratio, percent or per mille, of either sign; a negative slope is a descent. An angle must be strictly between −90° and 90°. At exactly ±90° the run is zero and the ratio is undefined, so that is refused rather than returned as an infinity.

## Approximations

Within the ratio family, none: the conversions are multiplications by 100 and 1,000. Between ratio and angle, tan and atan are not correctly rounded by any standard library, so the vectors allow a few units in the last place rather than the single rounding an exact ratio costs.

## Worked example

- sourcePublisher: Python Software Foundation (reference implementation); the definition of grade is standard
- sourceTitle: Python `math.tan` and `math.atan`, over the definition ratio = rise/run = tan(angle)
- sourceEdition: CPython 3.13
- sourceLocator: grade as a percentage is 100 times the tangent of the angle of inclination
- independent: yes
- inputs: 25 cases, including a 100% grade, a 1° slope, a −2.5% descent, a 8.333…% grade against its 1-in-12 ratio, and 90° refused
- outputs: all four representations in each case
- tolerance: 2e-15 relative, a few ulp of tan and atan
- verifiedBy: golden vectors v001 to v025, run by the core on every build
- verifiedOn: 2026-09-23

The reference is a different implementation of the same two functions: Python's `math` against the core's libm. That is a weaker independence claim than the exact-definition converters can make — there is no exact rational arithmetic to fall back on when the answer is transcendental — and it is stated rather than dressed up. What it does catch is a degree-radian confusion, an inverted ratio, a factor of 100 in the wrong place, and the percent-as-angle error, all of which are far larger than a few ulp.

## Differential tests

- `tools/vectors/gen_units.py` and `gen_units_special.py`: all 25 vectors, from Python's tan and atan
- `core/crates/gp-units/tests/units.rs`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/units.slope.convert.jsonl`: 25 vectors, every representation as input, both signs, and the refused vertical

## Invariants

- `core/crates/gp-units/tests/units.rs` `slope_invariants`: the ratio family is exact — a ratio of 0.25 is exactly 25% and exactly 250‰ — and the same value given in any of the three comes back identically; a 45° slope is exactly a ratio of 1 and exactly 100%, which is the assertion that catches percent being read as an angle; zero is flat in all four; a negative slope stays negative in all four; the round trip holds, a ratio to degrees and back landing within a couple of ulp; and ±90° is refused as out of domain while 89.999° is not, so the boundary is where it says it is
