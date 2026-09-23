<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Frequency converter (`units.frequency.convert`)

## Method

Every unit here is defined exactly in terms of the hertz, so a conversion is one multiplication by a ratio of two exact numbers: the value times the source unit's definition, divided by the target's, rounded once. Rounding once is the method. Converting through an intermediate unit in two steps rounds twice and loses a bit that a single ratio keeps.

The four ordinary units are decimal SI multiples: kilo, mega and giga are 10³, 10⁶ and 10⁹ exactly. They are not the binary 1,024-based prefixes some software uses for storage; a frequency prefix has never been binary, and nothing here treats it as such.

Two more units are for a different job: ppm/yr and ppb/yr, a fractional drift rate. An oscillator that gains a part per million every year is drifting at a rate that is a frequency — one over a time — and quoting it this way is how a clock or a reference specification does it. The year is the Julian year of exactly 365.25 days, 31,557,600 s, so a part per million per year is 1/(10⁶ · 31,557,600) Hz.

## Equations

- Conversion: y = x · (source unit in Hz) / (target unit in Hz).
- kHz: 10³ Hz. MHz: 10⁶ Hz. GHz: 10⁹ Hz — decimal, exactly.
- Julian year: 365.25 d = 31,557,600 s exactly.
- ppm/yr: 1/(10⁶ · 31,557,600) Hz. ppb/yr: a thousandth of that.

## Symbols and units

x is the frequency in the source unit and y in the target. The units are Hz, kHz, MHz, GHz, ppm/yr and ppb/yr.

## Domain

Any finite frequency. Negative values are accepted, which is what a drift rate downwards needs; a negative ordinary frequency is not a physical quantity, and the converter does not know which of the two it has been given.

## Approximations

None. Every definition is an exact integer ratio and the only inexactness is the single final rounding.

## Worked example

- sourcePublisher: National Institute of Standards and Technology; International Astronomical Union
- sourceTitle: Guide for the Use of the International System of Units (SI), NIST SP 811; the IAU definition of the Julian year
- sourceEdition: SP 811, 2008 edition, Table 5 (SI prefixes); Julian year, IAU (1976) System of Astronomical Constants
- sourceLocator: k = 10³, M = 10⁶, G = 10⁹ exactly; Julian year = 365.25 d = 31,557,600 s exactly
- independent: yes
- inputs: 40 conversions, including 2.4 GHz to MHz, 121.5 MHz to kHz, 433 MHz to Hz, and every ordered pair of the six units
- outputs: the converted value in each case
- tolerance: 5e-16 relative, at most two roundings of an exact ratio
- verifiedBy: golden vectors v001 to v040, run by the core on every build
- verifiedOn: 2026-09-23

The expected values come from `tools/vectors/gen_units.py` and `gen_units_gaps.py`, which restate each unit from its published definition and convert in exact rational arithmetic — Python `Fraction`, not floating point — rounding to binary64 once at the end. The reference therefore has no rounding error of its own, and the tolerance is set by what one correctly rounded ratio costs rather than by what two libraries happen to agree on.

The drift units had no vector at all until the gap sweep added them: the tool offered ppm/yr and ppb/yr and nothing had ever converted either. They are now covered against every other unit in both directions, which is what catches a year defined as 365 days instead of 365.25.

## Differential tests

- `tools/vectors/gen_units.py` and `gen_units_gaps.py`: all 40 vectors, from the exact definitions in rational arithmetic
- `core/crates/gp-units/tests/units.rs`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/units.frequency.convert.jsonl`: 40 vectors, the hand-picked cases and every ordered pair of the units, in both directions

## Invariants

- `core/crates/gp-units/tests/units.rs` `frequency_invariants`: every ordered pair round trips to the last bit, and a unit converted to itself is untouched; the conversion is linear, so zero stays zero, a sign is kept and twice the input is twice the output; a kilohertz is exactly 1,000 Hz, a megahertz 1,000 kHz and a gigahertz 1,000 MHz, so the prefixes are decimal and not the binary 1,024 that storage software uses; a part per million per year is exactly a thousand parts per billion per year; and the year behind them is the Julian one — a hertz comes to 31,557,600,000,000 ppm/yr, not the 31,536,000,000,000 of a 365-day year
