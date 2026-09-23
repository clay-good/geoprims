<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Part 107 maximum altitude (`drone.ops.part107-altitude`)

## Method

14 CFR 107.51(b), as printed on eCFR (current through 2026-09-21, last amended 2016-12-30): the small unmanned aircraft "cannot be higher than 400 feet above ground level," unless it is flown within a 400-foot radius of a structure and does not fly higher than 400 feet above the structure's immediate uppermost limit. The tool applies those two conditions to the structure and distance you give. With a ground elevation it adds the limit to it for a ceiling above mean sea level, and with a geoid height it converts that to a height above the ellipsoid, the datum many GPS autopilots fly. The limit value comes from `data/regulations.json` (row `faa-107-altitude`, reviewed 2026-09-18), and every result carries the review date and the eCFR link.

## Equations

- No structure, or more than 400 ft from it: max AGL = 400 ft.
- Within 400 ft of a structure of height h (distance ≤ 400 ft): max AGL = h + 400 ft.
- Max MSL = ground elevation + max AGL.
- Max HAE = max MSL + N, where N is the geoid height (ellipsoid minus mean sea level).

## Symbols and units

`structure_height` h and `structure_distance` are lengths in feet (any length unit is converted), measured from the ground at the structure's base and horizontally from the structure. `ground_elevation` is in feet above mean sea level; `geoid_height` N is in meters and is negative over most of the United States. Out come `max_agl`, `max_msl`, `max_hae` (all in feet), the `basis` in words, and the not-legal-advice `notice`.

## Domain

A structure height and distance of zero or more, given together; giving one without the other, or a negative value, is an input error. Exactly 400 ft from the structure counts as within its radius. A ground elevation can be below sea level. A geoid height is used only with a ground elevation.

## Approximations

None in the arithmetic. The rule is a regulation, not a model, so the answer is exact for the inputs given. What it leaves out: airspace access (Class B, C, D, and surface Class E airspace still need an FAA authorization under 107.41, whatever the altitude), waivers, TFRs, local restrictions, and recreational flying under 49 U.S.C. 44809. On sloping ground, the height above the ground under the drone can differ from the height above the structure's base. The MSL and HAE ceilings are only as good as the ground elevation and geoid height entered.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: 14 CFR Part 107, Small Unmanned Aircraft Systems, § 107.51 Operating limitations for small unmanned aircraft
- sourceEdition: eCFR, current through 2026-09-21 (section last amended 2016-12-30), read through the eCFR versioner API on 2026-09-23
- sourceLocator: § 107.51(b): "cannot be higher than 400 feet above ground level, unless the small unmanned aircraft: (1) Is flown within a 400-foot radius of a structure; and (2) Does not fly higher than 400 feet above the structure's immediate uppermost limit"
- independent: yes
- inputs: no structure nearby
- outputs: 400 ft above ground level, the figure the regulation prints
- tolerance: 0.5 ft (the rule prints whole feet)
- verifiedBy: golden vector v018, run by the core on every build
- verifiedOn: 2026-09-23

The regulation prints the answer for open country and the two conditions for a structure, but no worked structure case. The FAA guidance checked on 2026-09-23 has none with numbers either: AC 107-2A, the Part 107 final-rule preamble (81 FR 42064, June 28, 2016), AIM chapter 11-4, the Remote Pilot study guide, and the UAG sample questions. The structure vectors therefore apply the two printed conditions in Python rather than copying a published answer.

## Differential tests

- `tools/vectors/gen_hold_part107.py`: a separate Python reading of § 107.51(b) at 12 cases: a structure at 0 ft and 250 ft, the radius exactly at 400 ft and 0.1 ft beyond it, metric inputs exactly at 121.92 m (400 ft) and just past it, a half-mile structure, a below-sea-level ground elevation, and MSL and HAE ceilings with geoid heights from −30.5 m to +25 m (within 1e-9 relative). It also covers the rule's printed 400 ft, and four input errors.
- `core/vectors/drone.ops.part107-altitude.jsonl`: those 17 vectors and the 5 first-release vectors, run through the core on every build

## Invariants

- `core/crates/gp-drone/tests/power_ops.rs` `part107_altitude_invariants`: the limit is never below 400 ft, is exactly 400 ft beyond 400 ft from a structure, and is the structure height plus 400 ft at or within 400 ft, at 8 heights × 8 distances. The same inputs in meters give the same answer, the limit rises steadily with the structure height inside the radius, and the MSL and HAE ceilings are the AGL limit plus the ground elevation and then the geoid height. The test counts the 64 cases it reached.
