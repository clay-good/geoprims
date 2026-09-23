<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Angular rate converter (`units.angular-rate.convert`)

## Method

Each unit is defined in terms of degrees per second, and a conversion is one multiplication by that ratio, rounded once. Rounding once is the method. Converting through an intermediate unit in two steps rounds twice and loses a bit that a single ratio keeps.

Five of the six units are exact fractions of a turn over an exact time, so their ratios are exact rationals: a revolution per minute is 360°/60 s = 6 °/s, a degree per minute is 1/60 °/s. The radian is the exception. It is 180/π degrees, an irrational number, so a conversion touching rad/s carries a looser bound — the error of one binary64 value of π, not of the definition.

The two per-year units are for a different job. A tectonic plate rotation or a station velocity is published in milliarcseconds or arcseconds per year, and the year meant there is the Julian year of exactly 365.25 days — 31,557,600 s. Not the calendar year, which is 365 or 366 days and would move the answer by up to a fifth of a percent.

## Equations

- Conversion: y = x · (source unit in °/s) / (target unit in °/s).
- Revolution per minute: 6 °/s exactly. Degree per minute: 1/60 °/s exactly.
- Radian per second: 180/π °/s, irrational.
- Arcsecond per year: 1/(3600 · 31,557,600) °/s. Milliarcsecond per year: a thousandth of that.
- Julian year: 365.25 d = 31,557,600 s exactly.

## Symbols and units

x is the rate in the source unit and y in the target. The units are °/s, °/min, rad/s, rpm, arcsec/yr and mas/yr.

## Domain

Any finite rate. Negative values are accepted and meaningful: a left turn against a right one, or a rotation the other way about the same axis.

## Approximations

Only π. Conversions between the five exact units are exact to one rounding; a conversion touching rad/s is exact to the binary64 value of π, which the vectors allow for with a 2e-15 relative bound instead of 5e-16.

## Worked example

- sourcePublisher: National Institute of Standards and Technology; International Astronomical Union
- sourceTitle: Guide for the Use of the International System of Units (SI), NIST SP 811; the IAU definition of the Julian year
- sourceEdition: SP 811, 2008 edition, Appendix B.8; Julian year, IAU (1976) System of Astronomical Constants
- sourceLocator: degree = (π/180) rad; minute = 60 s; Julian year = 365.25 d = 31,557,600 s exactly, the year published station and plate velocities are quoted against
- independent: yes
- inputs: 40 conversions, including a standard-rate turn of 3 °/s to rpm, 1 rpm to °/s, 3 °/s to rad/s, and every ordered pair of the six units
- outputs: the converted value in each case
- tolerance: 5e-16 relative between exact units, 2e-15 where a radian is involved
- verifiedBy: golden vectors v001 to v040, run by the core on every build
- verifiedOn: 2026-09-23

The expected values come from `tools/vectors/gen_units.py` and `gen_units_gaps.py`, which restate each unit from its published definition and convert in exact rational arithmetic — Python `Fraction`, not floating point — rounding to binary64 once at the end. Only the radian leaves that arithmetic, and it is the one unit whose tolerance is looser.

The per-year units had no vector at all until the gap sweep added them: the tool offered arcsec/yr and mas/yr and nothing had ever converted either. They are now covered against every other unit in both directions, which is what catches a year defined as 365 days instead of 365.25.

## Differential tests

- `tools/vectors/gen_units.py` and `gen_units_gaps.py`: all 40 vectors, from the exact definitions in rational arithmetic
- `core/crates/gp-units/tests/units.rs`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/units.angular-rate.convert.jsonl`: 40 vectors, the hand-picked cases and every ordered pair of the units, in both directions

## Invariants

- `core/crates/gp-units/tests/units.rs` `angular_rate_invariants`: every ordered pair round trips to the last bit or two, and a unit converted to itself is untouched; the conversion is linear, so zero stays zero, a sign is kept and twice the input is twice the output; one rpm is exactly 6 °/s and one degree per minute exactly 1/60 °/s, to the last bit; a full turn per minute is exactly 6 °/s, which is the same statement arrived at the other way; an arcsecond per year is exactly a thousand milliarcseconds per year; and the year behind them is the Julian one — one degree per year comes to 3,600 arcsec/yr and the seconds behind it are 31,557,600, not the 31,536,000 of a 365-day year, a 0.07% difference that a loose check would miss
