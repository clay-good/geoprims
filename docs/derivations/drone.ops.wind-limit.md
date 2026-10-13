<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Wind limit at flying height (`drone.ops.wind-limit`)

## Method

A reported wind is measured at one height, 10 m for a METAR, and the wind is stronger higher up. The tool carries it to the flying height with the power law printed in NREL's Small Wind Site Assessment Guidelines (NREL/TP-5000-63696, section 5.3.2), using the exponent for the terrain: 1/7 (0.143) over flat, open terrain, as NREL's Wind Resource Assessment Handbook gives it (NREL/SR-440-22223, page 3-3), and otherwise the textbook exponent from Table 1 of the guidelines. A reported gust is scaled by the same factor. The limit is the drone's wind rating, or its maximum airspeed less a margin the reader chooses. The margin is the limit less the stronger of the wind and the gust at height, and the groundspeed into the wind is the maximum airspeed less the wind at height. The result names the exponent and says that gusts near buildings, trees, and terrain are not modeled.

## Equations

- V(h) = V_ref × (h / h_ref)^α
- G(h) = G_ref × (h / h_ref)^α
- Limit = wind rating, or V_max × (1 − m)
- Margin = limit − max(V(h), G(h)); groundspeed into the wind = V_max − V(h)

| Terrain | α | NREL source |
|---|---|---|
| water | 0.09 | Table 1, calm sea |
| open (default) | 1/7 = 0.143 | Handbook p. 3-3, flat open terrain |
| crops | 0.19 | Table 1, crops, tall-grass prairie |
| hedges | 0.24 | Table 1, scattered trees and hedges |
| suburbs | 0.31 | Table 1, city suburbs, villages, scattered forests |
| woodland | 0.43 | Table 1, woodlands |

## Symbols and units

V_ref and G_ref the reported wind and gust (any speed unit, knots by default); h_ref the report height (m, default 10 m) and h the flying height above the ground (m); α the shear exponent (a pure number, or the reader's own); V_max the maximum airspeed and m the margin share. Speeds come out in m/s.

## Domain

A wind of zero or more, and a gust at least the wind; heights above zero and up to 1,000 m; a wind rating, or an airspeed with a margin. At the report height the factor is 1 and the wind is unchanged.

## Approximations

The power law describes a steady, well-mixed wind over even ground. The guidelines warn that real exponents range from 0.2 to 0.5 with terrain and roughness, and the handbook that light winds and vegetation shear more than 1/7. Near buildings, trees, cliffs, and ridges the flow is not a profile at all. Scaling a gust by the same factor treats a moment as if it were a mean, a rough guide only.

## Worked example

- sourcePublisher: New York State Energy Research and Development Authority
- sourceTitle: Customer Sited Wind Handbook
- sourceEdition: Final Report 10-08, August 2010, read at the issuer on 2026-10-12
- sourceLocator: section 6.2, page 33, Wind Shear Adjustment example
- independent: yes
- inputs: 11.7 mph at 100 ft; α = 0.230, calculated in the source from 11.7 mph at 100 ft and 12.2 mph at 120 ft; estimate at 110 ft
- outputs: 12.0 mph at 110 ft. The tool gives 11.96 mph (5.346 m/s) before rounding.
- tolerance: 0.05 mph, half the source's printed 0.1 mph
- verifiedBy: golden vector v008, run by the core on every build
- verifiedOn: 2026-10-12

## Differential tests

- `tools/vectors/gen_mission_planning.py`: the power law in Python at seven computed cases, one per terrain class, the report height itself, a report height of 30 m, feet and knots, and a limit from an airspeed and a 50% margin (within 1e-12 relative), plus NYSERDA's published 12.0 mph example at its printed precision.

## Invariants

- `core/crates/gp-drone/tests/planning.rs` `wind_unchanged_at_report_height`: at the report height the wind and gust come out exactly as reported, for three terrains.
- `wind_rises_with_height_and_roughness`: at 15 pairings of height and terrain, the wind at height rises with height and with rougher terrain.
- `wind_gust_over_rating`: a gust over the rating with the sustained wind under it reports GUST_EXCEEDS_RATING with both values, and the sentence says gusts near obstacles are not modeled.
