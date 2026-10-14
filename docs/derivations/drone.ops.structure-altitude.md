<!-- Derivation note (trust/correctness-program layer A). -->
# Part 107 altitude near a structure (`drone.ops.structure-altitude`)

## Method

14 CFR 107.51(b) limits a small unmanned aircraft to 400 ft above ground level, unless it is flown within a 400 ft radius of a structure and no higher than 400 ft above the structure's immediate uppermost limit. The operation applies that sentence to a structure's height and the aircraft's horizontal distance from it. It shares one calculation path, and the same dated regulatory reference, with `drone.ops.part107-altitude`.

## Equations

- Within 400 ft of the structure: maximum altitude above ground = structure height + 400 ft
- Beyond 400 ft: maximum altitude above ground = 400 ft
- Maximum MSL altitude = ground elevation + maximum altitude above ground
- Maximum ellipsoid height = maximum MSL altitude + geoid height

## Symbols and units

Structure height above the ground at its base; horizontal distance from the structure; ground elevation above mean sea level; geoid height N (ellipsoid minus MSL). Lengths in feet or meters; the limit is reported in feet.

## Domain

A structure height and distance of zero or more. Exactly 400 ft from the structure counts as within the radius.

## Approximations

None in the arithmetic. The ground under the aircraft is taken to be at the entered elevation, and the rule is read as written; airspace authorizations and waivers can set other limits.

## Worked example

- sourcePublisher: U.S. Government Publishing Office, Electronic Code of Federal Regulations
- sourceTitle: 14 CFR 107.51, Operating limitations for small unmanned aircraft
- sourceEdition: As dated in `data/regulations.json`
- sourceLocator: Paragraph (b), (b)(1), and (b)(2); the numbers below are this note's own arithmetic on the rule, not a published example
- independent: no
- inputs: a 1,150 ft tower, 250 ft away
- outputs: within the 400 ft radius, so 1,150 + 400 = 1,550 ft above ground
- tolerance: exact
- verifiedBy: golden vector v001 and the `focused_mission_and_ops_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_drone.py` `structure_altitude_vectors`: 16 random structures in feet and meters, with and without a ground elevation, by a separate Python statement of the rule.
- `core/vectors/drone.ops.structure-altitude.jsonl`: those vectors, the 400 ft and 401 ft edge, and two refusal cases.
- The integration suite requires the whole result to equal `drone.ops.part107-altitude` for the same inputs.

## Invariants

- The limit is never below 400 ft above ground.
- Inside the radius it rises foot for foot with the structure's height.
- Outside the radius the structure's height does not matter.
