<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Aircraft approach category (`aviation.ifr.approach-category`)

## Method

14 CFR 97.3 sorts airplanes into approach categories A to E by speed. The speed is VREF, or 1.3 × VS0 at maximum certificated landing weight when VREF is not specified. The tool takes VREF when given, else 1.3 × VS0, and finds the highest category whose lower bound the speed meets. The lower bounds of B to E are read from `data/regulations.json` (entries `faa-97-3-category-b` to `faa-97-3-category-e`), not written in the code. The result names the speed used, its basis, the band as text, and the rules' review date.

## Equations

- Speed: V = VREF, else V = 1.3 × VS0.
- Category: A if V < 91 kt; B if 91 ≤ V < 121; C if 121 ≤ V < 141; D if 141 ≤ V < 166; E if V ≥ 166.
- Bounds (from `data/regulations.json`, effective 2016-12-30, reviewed 2026-10-19): B 91 kt, C 121 kt, D 141 kt, E 166 kt.

## Symbols and units

VREF, VS0, and the speed used in knots by default (any speed unit accepted; the speed used is shown to one decimal). The category is one letter. The "rules as of" text cites 14 CFR 97.3 with the review date of the Category C entry.

## Domain

VREF or VS0, else INVALID_INPUT at `/vref`. The speed used must be finite and > 0, else INVALID_INPUT at `/vref` (or `/vs0` when VS0 was used). There is no upper bound; any speed at or above 166 kt is Category E. When both speeds are given, VREF is used.

## Approximations

None in the lookup; it is exact to the regulation's bands. The category comes from certificated speeds at maximum certificated landing weight, not today's weight. An approach flown faster than the category's band uses the minimums of the faster speed's category, which the sentence says. Helicopters and some operators use other rules. A dated reference, not legal advice.

## Worked example

- sourcePublisher: Federal Aviation Administration (eCFR)
- sourceTitle: 14 CFR 97.3, Symbols and terms used in procedures
- sourceEdition: current eCFR text (amended 2016-12-30), read through the eCFR versioner API on 2026-10-19
- sourceLocator: Definition of aircraft approach category, Category C: 121 knots or more but less than 141 knots
- independent: yes
- inputs: VREF 121 kt
- outputs: Category C, speed used 121 kt, basis VREF
- tolerance: exact
- verifiedBy: golden vector v001 and `core/crates/gp-aviation/tests/approach.rs` `approach_category_invariants`
- verifiedOn: 2026-10-20

## Differential tests

- No generator script in `tools/vectors/` names this tool. The vectors' source field is 14 CFR 97.3 (eCFR, read 2026-10-19), aircraft approach category bands; the expected categories are read from the regulation's bands.
- `core/vectors/aviation.ifr.approach-category.jsonl`: 24 vectors. Both sides of every band edge (v001 to v008); a speed inside each category (v009 to v014); six VS0 cases through 1.3 × VS0, with 70 kt landing on 91.0 kt and 93 kt on 120.9 kt; a VREF in mph (v021); VREF winning over VS0 (v022); and two INVALID_INPUT refusals: no speed and a zero VREF.

## Invariants

- `core/crates/gp-aviation/tests/approach.rs` `approach_category_invariants`: each side of every band edge (90.9 A, 91 B, 120.9 B, 121 C, 140.9 C, 141 D, 165.9 D, 166 E); the category never goes down as VREF rises from 40 to 259 kt; VS0 70 kt gives the same category as VREF 91 kt; and with VREF 121 kt and VS0 50 kt, VREF decides (C).
