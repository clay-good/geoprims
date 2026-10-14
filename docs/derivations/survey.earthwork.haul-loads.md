<!-- Derivation note (trust/correctness-program layer A). -->
# Truck loads to haul a cut (`survey.earthwork.haul-loads`)

## Method

Soil takes up more room once it is dug. The bank volume from a takeoff is multiplied by one plus the swell to get the loose volume that has to be trucked, and that is divided by what one truck carries. A part load still needs a truck, so the count is rounded up. The operation shares its loose volume and load count with `survey.earthwork.shrink-swell`.

## Equations

- Loose volume = bank volume × (1 + swell / 100)
- Loads = ⌈loose volume ÷ truck capacity⌉
- Last load = loose volume − (loads − 1) × truck capacity

## Symbols and units

Bank and loose volumes and the truck capacity in any volume unit (cubic yards, cubic meters, cubic feet); they need not match. Swell is a percent. The last load is reported in the truck capacity's unit.

## Domain

A bank volume of zero or more, a swell from the input's declared range, and a positive truck capacity. A zero volume takes no loads.

## Approximations

The round-up allows one part in 10¹² for binary round-off, so a loose volume that is an exact multiple of the capacity does not gain a truck. Capacity is treated as volume only; legal weight limits are not modeled.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: add-survey-suite specification
- sourceEdition: 2026
- sourceLocator: survey/earthwork-and-grade, scenario "Swell to truck loads"; the arithmetic is checked by hand, not taken from a published table
- independent: no
- inputs: 1,000 bank yd³, 25% swell, 12 yd³ trucks
- outputs: 1,250 loose yd³, 105 loads (104.17 rounded up), last load 2 yd³
- tolerance: exact on the count
- verifiedBy: golden vector v002 and the `named_earthwork_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_survey.py` `haul_loads`: 16 random hauls across cubic yards, cubic meters, and cubic feet, evaluated in exact fractions so the expected round-up does not depend on floating point.
- `core/vectors/survey.earthwork.haul-loads.jsonl`: those vectors, the spec scenario, two exact-multiple cases, and two refusal cases.
- The integration suite requires the same loose volume and load count as `survey.earthwork.shrink-swell`.

## Invariants

- (loads − 1) × capacity < loose volume ≤ loads × capacity.
- The last load is more than zero and no more than one truck.
- An exact multiple of the capacity takes exactly that many loads, with a full last truck.
