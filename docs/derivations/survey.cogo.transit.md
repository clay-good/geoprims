<!-- Derivation note (trust/correctness-program layer A). -->
# Transit-rule traverse adjustment (`survey.cogo.transit`)

## Method

The operation fixes the shared traverse engine to the transit rule. It computes each course's unadjusted latitude and departure, the component sums, the closure magnitude, total length, and precision. Latitude misclosure is distributed according to the absolute latitudes, while departure misclosure is distributed according to the absolute departures. Adjusted components are accumulated from the entered starting coordinates.

## Equations

For course i:

- Li = di cos αi and Di = di sin αi
- latitude correction cli = −ΣL |Li| / Σ|L|
- departure correction cdi = −ΣD |Di| / Σ|D|
- adjusted components are L'i = Li + cli and D'i = Di + cdi

The component corrections sum to the negative misclosures when their corresponding denominator is nonzero.

## Symbols and units

The north azimuth α is clockwise from north. The course distance di and all component, correction, and coordinate values use one common linear unit. ΣL and ΣD are the unadjusted component misclosures. The vertical bars mean absolute value, not course length.

## Domain

The input is a closed plane traverse with 2 to 2,000 positive horizontal distances. Directions must share one grid or assumed north reference. Linear inputs must use compatible units, and international feet cannot be mixed with US survey feet. The rule is meaningful only when the governing survey procedure calls for transit adjustment.

## Approximations

The transit rule is a conventional adjustment, not least squares, and does not locate blunders. Its corrections depend on the orientation of the coordinate axes because rotating the traverse changes the absolute latitudes and departures. Course directions and distances must already be reduced to the working plane.

## Worked example

- sourcePublisher: GeoPrims project
- sourceTitle: add-survey-suite transit-rule scenario
- sourceEdition: 2026-10-13
- sourceLocator: traverse-adjustments requirement
- independent: no
- inputs: four courses at azimuths 45°, 135°, 225°, and 315.01° with lengths near 1,000 ft
- outputs: the adjusted coordinates close exactly and the result states that the correction depends on coordinate orientation
- tolerance: 1 part in 1 billion with a 1e-9 ft absolute allowance
- verifiedBy: golden vector v001 and named_traverse_adjustments_match_the_general_tool
- verifiedOn: 2026-10-13

## Differential tests

- `core/vectors/survey.cogo.transit.jsonl` compares 20 successful traverses with an independent Python implementation of the transit-rule equations and includes unit, direction, and distance refusal cases.

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `named_traverse_adjustments_match_the_general_tool` requires the named operation to match the general traverse tool with `adjustment` fixed to `transit`.
- `core/crates/gp-survey/tests/survey.rs` `traverse_closure_invariants` proves exact adjusted closure while retaining the rule's documented orientation dependence.
