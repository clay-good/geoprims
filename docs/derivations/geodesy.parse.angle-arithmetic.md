<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Angle arithmetic in degrees, minutes and seconds (`geodesy.parse.angle-arithmetic`)

## Method

Adding and subtracting angles written as degrees, minutes and seconds is the kind of arithmetic that is easy by hand and easy to get wrong by machine, because the carries are sexagesimal in two places and decimal in a third.

The method removes the problem rather than handling it: every term is converted to **arcseconds**, a single number, summed with its sign, normalized if asked, and written back as D°MM'SS.ss" at the end. In one number there are no carries to lose. The carries reappear only in the final formatting, and they are done in the right order — the printed seconds are rounded first, and if that rounding reaches 60 it carries into the minutes, and if the minutes reach 60 they carry into the degrees.

That last step is the one worth stating. 59'59.995" printed to two decimals is not 59'60.00"; it is one whole degree. A formatter that rounds the seconds field on its own produces the first, which is not a time anyone writes and not an angle anyone means.

Normalization is optional and has two forms: to [0, 360), the convention for a bearing, or to (−180, 180], the convention for a signed difference.

## Equations

- Each term: s = 3600·D + 60·M + S, negated when subtracting.
- Sum: total = Σ sᵢ.
- Normalize to 0–360: total mod 1,296,000 arcseconds.
- Normalize to ±180: ((total + 648,000) mod 1,296,000) − 648,000.
- Format: round to the printed decimal, then carry 60 → 1 into minutes and 60 → 1 into degrees.

## Symbols and units

`terms` is a list, each with an `angle` — `D-M-S`, `D M S`, or decimal degrees — and an optional `operation` of `subtract`. `normalize` is `none`, `0-360` or `plus-minus-180`. Out come `dms`, `degrees` and `seconds`.

## Domain

Any finite angles, any number of terms. Without normalization the sum can exceed a turn, which is correct for a traverse closure or an accumulated turn.

## Approximations

The arithmetic is exact to double precision in arcseconds — a number small enough that an angle of a full turn is 1,296,000, so no precision is lost anywhere an angle is used. The only rounding is the printed one.

## Worked example

- sourcePublisher: the sexagesimal definition of the degree
- sourceTitle: degrees, minutes and seconds — 60 arcminutes to a degree, 60 arcseconds to an arcminute
- sourceEdition: definition
- sourceLocator: 1° = 60′ = 3600″
- independent: yes
- inputs: 24 sums, including three one-second terms, an addition that carries through seconds and minutes into a whole degree, a subtraction that borrows, a mixed decimal and DMS pair, a negative term, both normalizations, and 359°59'59.995" + 0.005"
- outputs: the DMS string, the decimal degrees and the arcseconds
- tolerance: exact on the string, 1e-9″ on the total
- verifiedBy: golden vectors v001 to v024, run by the core on every build
- verifiedOn: 2026-09-23

The reference in `tools/vectors/gen_parse_more.py` does the same arithmetic in Python `Fraction`, so the carry is exact rather than nearly exact, and the formatting is done by rounding the whole angle to the printed unit before splitting it into components — a different route to the same answer than carrying between fields. The case that separates the two is 359°59'59.995" plus five thousandths of a second, normalized to 0–360: it is exactly zero, and a formatter that carried wrongly would print 359°59'60.00" or 360°00'00.00".

## Differential tests

- `tools/vectors/gen_parse_more.py`: 16 of the 24 vectors, in exact rational arithmetic
- `core/crates/gp-geodesy/tests/`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/geodesy.parse.angle-arithmetic.jsonl`: 24 sums across both normalizations

## Invariants

- `core/crates/gp-geodesy/tests/geodesy.rs` `angle_arithmetic_invariants`: the three outputs agree — the arcseconds are 3,600 times the degrees, and the DMS string parses back to the same angle; adding an angle and subtracting it again returns the original exactly; the carry reaches all the way, so 59'59.995" prints as a whole degree and never as 60 minutes; the two normalizations agree where they overlap, a result in [0, 180] being the same in both; normalizing to 0–360 always gives a value in that range, and to ±180 a value in (−180, 180]; and the sum is order-independent, since the terms are added in arcseconds
