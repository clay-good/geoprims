<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Data rate converter (`units.data-rate.convert`)

## Method

Every unit here is a decimal SI multiple of the bit per second, so a conversion is one multiplication by a ratio of two exact integers: the value times the source unit's definition, divided by the target's, rounded once. Rounding once is the method. Converting through an intermediate unit in two steps rounds twice and loses a bit that a single ratio keeps.

The one decision worth stating is that the prefixes are decimal. A megabit per second is 10⁶ bit/s, not 2²⁰. This is not a convention picked here: IEC 80000-13 and the SI both give k, M and G their decimal meanings and provide Ki, Mi and Gi for the binary ones, and network and link rates have always been quoted decimally — a 100 Mbit/s link carries 100,000,000 bits a second. The gap between the two readings is 4.9% at mega and 7.4% at giga, which is large enough to matter and small enough to go unnoticed.

Bits are also not bytes. Nothing here divides by eight; a rate in bytes per second is a different quantity, and a converter that silently switched between them would be the worst kind of wrong.

## Equations

- Conversion: y = x · (source unit in bit/s) / (target unit in bit/s).
- kbit/s: 10³ bit/s. Mbit/s: 10⁶ bit/s. Gbit/s: 10⁹ bit/s — decimal, exactly.

## Symbols and units

x is the rate in the source unit and y in the target. The units are bit/s, kbit/s, Mbit/s and Gbit/s.

## Domain

Any finite rate. Negative values are accepted arithmetically — a difference of two rates is signed — though a negative throughput is not a thing.

## Approximations

None. Every definition is an exact power of ten and the only inexactness is the single final rounding.

## Worked example

- sourcePublisher: International Electrotechnical Commission; National Institute of Standards and Technology
- sourceTitle: IEC 80000-13, Quantities and units — Part 13: Information science and technology; Guide for the Use of the International System of Units (SI), NIST SP 811
- sourceEdition: IEC 80000-13:2008; SP 811, 2008 edition, Table 5 (SI prefixes)
- sourceLocator: k = 10³, M = 10⁶, G = 10⁹ exactly; the binary multiples are the separate prefixes Ki, Mi and Gi
- independent: yes
- inputs: 22 conversions, including 20 Mbit/s to kbit/s, 1 Gbit/s to Mbit/s, 56 kbit/s to bit/s, and every ordered pair of the four units
- outputs: the converted value in each case
- tolerance: 5e-16 relative, at most two roundings of an exact ratio
- verifiedBy: golden vectors v001 to v022, run by the core on every build
- verifiedOn: 2026-09-23

The expected values come from `tools/vectors/gen_units.py`, which restates each unit from its published definition and converts in exact rational arithmetic — Python `Fraction`, not floating point — rounding to binary64 once at the end. Every value in this family is an integer power of ten times the input, so each conversion is exact in binary64 across the whole range the tool accepts, and the tolerance is there for form rather than because anything needs it.

## Differential tests

- `tools/vectors/gen_units.py`: all 22 vectors, from the exact definitions in rational arithmetic
- `core/crates/gp-units/tests/units.rs`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/units.data-rate.convert.jsonl`: 22 vectors, the hand-picked cases and every ordered pair of the units, in both directions

## Invariants

- `core/crates/gp-units/tests/units.rs` `data_rate_invariants`: every ordered pair round trips to the last bit, and a unit converted to itself is untouched; the conversion is linear, so zero stays zero, a sign is kept and twice the input is twice the output; the prefixes are decimal — a kbit/s is exactly 1,000 bit/s, a Mbit/s exactly 1,000 kbit/s and a Gbit/s exactly 1,000 Mbit/s — and each is separately asserted *not* to be its binary counterpart, 1,024, 1,048,576 or 1,073,741,824, since those are the values a converter written against the wrong convention would carry
