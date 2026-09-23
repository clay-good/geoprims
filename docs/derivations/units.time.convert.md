<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Time converter (`units.time.convert`)

## Method

Every unit here is defined exactly in terms of the SI unit, so a conversion is one multiplication by a ratio of two exact numbers: the value times the source unit's definition, divided by the target's, rounded once. Rounding once is the method. Converting through an intermediate unit in two steps rounds twice and loses a bit that a single ratio keeps.

Every unit here is an exact whole number of seconds or an exact power of ten of one, so the conversions are the simplest in the catalog. What needs saying is not how it converts but what it converts: a duration, a span of elapsed time, and never a moment or a date.

## Equations

- Conversion: y = x · (source unit in seconds) / (target unit in seconds).
- Minute: 60 s. Hour: 3,600 s. Day: 86,400 s. Millisecond: 1/1,000 s.
- All exact, and all independent of any calendar.

## Symbols and units

x is the duration in the source unit and y in the target. The units are s, ms, min, h and d.

## Domain

Any finite duration of either sign. A negative duration is an interval measured backwards and converts like any other.

## Approximations

None at all: every ratio is a whole number or a power of ten, and the single final rounding is exact for any value a double can hold to that precision.

## Worked example

- sourcePublisher: National Institute of Standards and Technology
- sourceTitle: Guide for the Use of the International System of Units (SI), NIST SP 811; and NIST Handbook 44
- sourceEdition: SP 811, 2008 edition, Appendix B.8
- sourceLocator: minute = 60 s; hour = 3 600 s; day = 86 400 s
- independent: yes
- inputs: 22 conversions, including 90 minutes to hours, 1 day to seconds, 1,500 ms to seconds, and every ordered pair of the five units
- outputs: the converted value in each case
- tolerance: 5e-16 relative, at most two roundings of an exact ratio
- verifiedBy: golden vectors v001 to v022, run by the core on every build
- verifiedOn: 2026-09-23

The expected values come from `tools/vectors/gen_units.py`, which restates each unit from its published definition and converts in exact rational arithmetic — Python `Fraction`, not floating point — rounding to binary64 once at the end. The reference therefore has no rounding error of its own, and the tolerance is set by what one correctly rounded ratio costs rather than by what two libraries happen to agree on.

A day here is exactly 86,400 seconds because that is what a duration of one day means. A particular calendar day may not be: one containing a leap second is 86,401 seconds long, and one containing a change of clocks is an hour longer or shorter. Nothing in this tool knows which day you mean, which is why it converts spans and the time domain handles dates.

## Differential tests

- `tools/vectors/gen_units.py`: all 22 vectors, from the exact definitions in rational arithmetic
- `core/crates/gp-units/tests/units.rs`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/units.time.convert.jsonl`: 22 vectors, the hand-picked cases and a sweep of every ordered pair

## Invariants

- `core/crates/gp-units/tests/units.rs` `time_invariants`: every ordered pair round trips exactly — these ratios are whole numbers, so the round trip is not merely close but identical; a unit converted to itself is untouched; the conversion is linear, so zero stays zero, a sign is kept and twice the input is twice the output; the chain holds exactly, a day being 24 hours, 1,440 minutes and 86,400 seconds; and a millisecond is exactly a thousandth of a second
