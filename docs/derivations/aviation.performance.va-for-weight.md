<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Maneuvering speed at a weight (`aviation.performance.va-for-weight`)

## Method

The flight manual gives maneuvering speed (VA) at maximum weight. A lighter airplane stalls at a lower speed, so its VA is lower too. The tool scales the published VA by the square root of the weight ratio. The relation comes from 14 CFR 23.335(c) in the text in force until the 2017 Part 23 rewrite (VA not less than VS √n), with the stall speed VS proportional to √W. The section was removed on 2017-08-30; it is cited for the relation, not as a current rule. When the clean stall speed is given, the tool also shows the handbook cross-check VA ≈ 1.7 × VS.

## Equations

- Weight ratio: r = W ÷ Wmax.
- Maneuvering speed: VA = VA,max × √r.
- Cross-check, when VS is given: 1.7 × VS.

## Symbols and units

VA,max, VA, and VS in knots by default (any speed unit accepted); W and Wmax in pounds by default (any mass unit). The ratio r is a pure number from 0 to 1. The output VA is shown to whole knots and r to four significant figures.

## Domain

VA,max > 0, else INVALID_INPUT at `/va_max`. Both weights > 0, else INVALID_INPUT at `/weight`. The weight now may not exceed the maximum weight, else OUT_OF_DOMAIN at `/weight`, since the formula applies at or below maximum weight. The stall speed is optional; when given it must be more than zero, else INVALID_INPUT at `/vs`.

## Approximations

The square-root rule follows from the stall speed changing with weight. It is not a certified value, and the flight manual governs where it lists VA by weight. The 1.7 × VS figure is a rule of thumb shown for comparison only; the tool does not use it to set VA. Full control deflections can still overstress the airplane at or below VA in some cases, so the result is a limit, not a target.

## Worked example

- sourcePublisher: Federal Aviation Administration (14 CFR 23.335(c), eCFR), relation only
- sourceTitle: 14 CFR 23.335, Design airspeeds
- sourceEdition: text in force until 2017-08-30 (removed by the Part 23 rewrite), read at 2017-01-01 through the eCFR versioner API
- sourceLocator: Paragraph (c), design maneuvering speed: VA not less than VS √n; the numbers are the add-aviation-practical-additions design A7 example
- independent: no
- inputs: VA 105 kt at a maximum weight of 2,550 lb, flying at 2,000 lb
- outputs: VA 92.99 kt (92.98956298171001), ratio 0.7843137254901961
- tolerance: 1e-12 relative, 1e-9 absolute in the vector; the test holds 92.99 kt within 0.01 kt
- verifiedBy: golden vector v001 and `core/crates/gp-aviation/tests/physiology.rs` `va_for_weight_invariants`
- verifiedOn: 2026-10-20

The regulation states the relation, not this example. The 92.99 kt is derived from it, not printed in a published source.

## Differential tests

- No generator script in `tools/vectors/` names this tool. The vectors' source field says they were derived from 14 CFR 23.335(c) (pre-2017) with VS proportional to √W and worked in Python. The arithmetic is one square root, so this checks unit handling and the refusals, not a separate method.
- `core/vectors/aviation.performance.va-for-weight.jsonl`: 21 vectors. The design example (v001); VA at maximum weight equal to the published VA (v002); 15 more airplanes, 8 of them with a stall speed for the 1.7 × VS cross-check; one in kilograms (v018); and three refusals: above maximum weight (OUT_OF_DOMAIN), zero weight, and zero VA (INVALID_INPUT).

## Invariants

- `core/crates/gp-aviation/tests/physiology.rs` `va_for_weight_invariants`: at 2,550 lb the result is the published 105 kt; at 2,000 lb it is 92.99 kt within 0.01; VA at 1,800 lb is below VA at 2,200 lb; and a quarter of the maximum weight gives half the VA, 52.5 kt.
