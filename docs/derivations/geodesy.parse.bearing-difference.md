<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Bearing difference (`geodesy.parse.bearing-difference`)

## Method

The smallest signed turn from one bearing to another. Subtracting two bearings is the obvious thing and the wrong thing: 350° to 10° is a 20° turn to the right, not a 340° turn to the left, and the naive difference gives the second answer every time a heading crosses north.

The fix is one expression: ((to − from + 180) mod 360) − 180. Adding 180 before the modulus and subtracting it after moves the wrap from 0° — where bearings are dense — to ±180°, where the two answers are the same turn taken the other way round. The result is in [−180, 180): positive is a right turn, negative a left. The half-open end falls out of the formula rather than being chosen — a difference of exactly 180 gives (360 mod 360) − 180 = −180 — so an exact reversal reads as a left turn.

The modulus is the mathematical one, not the C remainder. A negative dividend must come back positive, so `(-10) mod 360` is 350 and not −10; using a remainder that keeps the sign gives the wrong turn for exactly half the inputs.

## Equations

- difference = ((to − from + 180) mod 360) − 180, with mod the non-negative remainder.
- Range: [−180, 180). An exact reversal comes out as −180, which is what the expression gives; it is a choice made by the definition, not on top of it.

## Symbols and units

`from` and `to` are bearings in degrees, in any range. Out comes `difference` in degrees, signed.

## Domain

Any two finite bearings, including ones outside 0–360, which are reduced first.

## Approximations

None. The expression is exact arithmetic; the only rounding is the one the inputs already carry.

## Worked example

- sourcePublisher: the definition of a signed angular difference
- sourceTitle: ((to − from + 180) mod 360) − 180
- sourceEdition: definition
- sourceLocator: the standard wrap-to-(−180, 180] expression
- independent: yes
- inputs: 23 pairs, including north to north, the four quadrants, a pair either side of north, an exact reversal in both directions, and headings a half degree apart across the wrap
- outputs: the signed difference in each case
- tolerance: 1e-12°
- verifiedBy: golden vectors v001 to v023, run by the core on every build
- verifiedOn: 2026-09-23

The reference in `tools/vectors/gen_parse_more.py` evaluates the same expression in exact rationals rather than in floating point, so the answer at ±180 is decided by the definition rather than by what a double happens to do there. Writing the invariants I assumed the other end was open and had to be corrected by the formula: 359.5° to 179.5° is −180, not +180. The cases that matter are the ones where a plain subtraction is wrong: 359° to 1° is +2 and not −358; 1° to 359° is −2 and not +358.

## Differential tests

- `tools/vectors/gen_parse_more.py`: 17 of the 23 vectors, in exact rational arithmetic
- `core/crates/gp-geodesy/tests/`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/geodesy.parse.bearing-difference.jsonl`: 23 vectors around the whole circle

## Invariants

- `core/crates/gp-geodesy/tests/geodesy.rs` `bearing_difference_invariants`: every answer is within [−180, 180), which is the range the expression produces rather than one imposed on it; reversing the arguments negates the answer, except at the exact reversal, where both directions read −180; a bearing to itself is zero, and so is a bearing to itself plus a whole turn; the difference across north is small and signed the short way, 359° to 1° being +2 rather than −358; adding a full turn to either bearing changes nothing; and the answer added to `from` gives `to` back, modulo 360, which is what makes it a turn rather than a number
