<!-- Derivation note (trust/correctness-program layer A). -->
# Bowditch traverse adjustment (`survey.cogo.bowditch`)

## Method

The operation runs the compass, or Bowditch, branch of the same traverse engine used by `survey.cogo.traverse-closure`. It first resolves every bearing or azimuth into a north azimuth and each horizontal distance into one common linear unit. It computes the unadjusted latitudes, departures, closure vector, total length, and precision before distributing the closure correction by course length. The final point is set exactly to the entered start after the adjusted components have been accumulated, removing only floating-point closure residue.

## Equations

For course i with length di, latitude Li, departure Di, total length T, and component misclosures ΣL and ΣD:

- Li = di cos αi and Di = di sin αi
- latitude correction cli = −ΣL(di/T)
- departure correction cdi = −ΣD(di/T)
- adjusted latitude L'i = Li + cli
- adjusted departure D'i = Di + cdi

The corrections sum to −ΣL and −ΣD, so the adjusted components close.

## Symbols and units

The north azimuth α is clockwise from north. Lengths, latitudes, departures, corrections, and coordinates use the common input linear unit. T is the sum of all course lengths. A precision such as 1:5,175 is T divided by the magnitude of the unadjusted closure vector. A perfect closure has no finite precision ratio and is reported explicitly.

## Domain

The input is a closed plane traverse with 2 to 2,000 positive horizontal course distances. Every direction must use the same grid or assumed north reference. All linear values must use compatible units; international feet and US survey feet cannot be mixed. Starting coordinates default to 5,000, 5,000 in the course unit when omitted.

## Approximations

Bowditch adjustment is a conventional error-distribution rule, not a stochastic least-squares adjustment. It assumes angular and distance observations have roughly comparable relative precision and does not identify blunders. The tool treats each course as a straight line on a plane and does not reduce slope distances or apply grid-to-ground corrections.

## Worked example

- sourcePublisher: University of Memphis
- sourceTitle: CIVL 1112 Surveying, Traverse Calculations
- sourceEdition: course notes retrieved 2026-09-19
- sourceLocator: page 4, compass-rule balanced latitude and departure table
- independent: yes
- inputs: five courses from AB through EA, starting at northing 0 and easting 0
- outputs: misclosure 0.182 ft, precision 1:5,175, and adjusted B through E coordinates matching the published table within 0.0015 ft
- tolerance: 0.0015 ft for the published rounded components and coordinates
- verifiedBy: golden vector v001
- verifiedOn: 2026-10-13

## Differential tests

- `core/vectors/survey.cogo.bowditch.jsonl` compares 20 successful traverses with an independent Python implementation of the compass-rule equations, including the published Memphis table, plus refusal cases.

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `named_traverse_adjustments_match_the_general_tool` requires the named operation to return the same result, display values, and warnings as the stable general tool with `adjustment` fixed to `compass`.
- `core/crates/gp-survey/tests/survey.rs` `traverse_closure_invariants` proves the adjusted traverse closes to its start, rotation preserves the closure magnitude, and reversing the loop negates both component sums.
