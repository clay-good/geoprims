<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Dip of the horizon (`navigation.los.dip`)

## Method

From a height of eye h the line of sight that just grazes the sea is tangent to the Earth, and the angle between it and true horizontal is the dip. On a sphere of radius R the tangent condition is exact: the cosine of the dip is R/(R + h). Refraction bends the ray down toward the surface, which is carried the way every line-of-sight calculation carries it, by replacing R with an effective radius R/(1 − k) and keeping the geometry exact. The navigator's rule of thumb, 1.76′ times the square root of the height in meters, is reported beside it with the difference between them.

## Equations

- Dip: δ = arccos( Rₑ / (Rₑ + h) ), with Rₑ = R/(1 − k).
- Small-height form, which is what the rules of thumb are: δ ≈ √(2h/Rₑ).
- Rule of thumb: δ ≈ 1.76′ · √(h in meters).
- Tied to the horizon distance, since the tangent point is Rₑ·δ along the surface: d = Rₑ · δ, with δ in radians.

## Symbols and units

h is the height of eye above the surface in meters, δ the dip in arcminutes, R the Earth's radius (6,371,000 m unless given), k the refraction coefficient (0.13 unless given), and Rₑ the effective radius. The dip is positive: the horizon is always below true horizontal.

## Domain

Any height of eye above the surface, from a person standing on a deck to an aircraft. The model is a sphere, so it does not distinguish latitude or the direction of sight, and it assumes the horizon is the sea. Over land the visible horizon is terrain instead, and the answer does not apply; the tool attaches `TERRAIN_NOT_CONSIDERED` rather than pretending otherwise. A refraction coefficient of 1 or more would put the effective radius at infinity or negative, and is refused.

## Approximations

The geometry is exact, not a series: the arccosine is evaluated directly rather than through √(2h/Rₑ), which would itself have been good to 0.07% at 10 km with no refraction, and 0.06% with it — the choice buys exactness, not a rescue. The one modeling choice is k, and it is the whole uncertainty. Abnormal refraction over water moves the real dip by several arcminutes, which dwarfs everything else here, so the answer is a nominal-atmosphere figure and not a measurement.

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: The American Practical Navigator (Bowditch), NGA Pub. 9
- sourceEdition: 2024 edition, Volume II
- sourceLocator: Table 12 (distance of the horizon, 1.17 √h in feet) together with the printed dip rule 1.76′ √h in meters
- independent: yes
- inputs: height of eye 10 m, k = 0.1689
- outputs: dip 5.5528′, against the rule's 5.5656′
- tolerance: 0.25%, which is how closely a square-root rule can track an arccosine
- verifiedBy: golden vector v023, run by the core on every build
- verifiedOn: 2026-09-23

Bowditch prints two rules, and the check is that they are the same rule. Table 12 gives the horizon distance as 1.17 √h with h in feet, and the dip rule gives 1.76′ √h with h in meters. Since d = Rₑ·δ, a distance rule and a dip rule together pin the effective radius they were both built on: 1.17 · 1852 / √0.3048 meters per √meter divided by 1.76′ in radians gives Rₑ = 7,666,208 m, which is k = 0.1689. Neither number was chosen to make that work, and there is no reason two separately printed navigator's rules should agree on a refraction coefficient unless both descend from the same physics. At that coefficient the exact dip at 10 m is 5.5528′ against the rule's 5.5656′, and the rule runs 0.231% high at every height from 1 m to 100 m — the same figure to three decimals, because both sides scale as √h and only the coefficient differs.

This also settles what the tool's rule difference means. At the default k = 0.13 the dip at 10 m is 5.6813′ and the rule is 5.5656′, a gap of −2.04%; at k = 0.1689 the gap is +0.231%. So about nine tenths of the reported difference is the refraction coefficient rather than the square root, which is worth knowing before reading it as the rule's error. The default is left alone — it is the coefficient the rest of the line-of-sight tools use, and changing it would move published results — but the limitations now say so.

## Differential tests

- `core/crates/gp-navigation/tests/horizon_parity.rs` `the_rule_matches_bowditch_table_12`: all 126 printed heights of Table 12, which is the distance form of the same rule this tool prints as a dip
- `tools/vectors/gen_los.py`: 22 vectors over heights from a person's eye to cruising altitude
- `core/vectors/navigation.los.dip.jsonl`: 23 vectors, the last from Bowditch's own pair of rules

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `dip_invariants`: the dip is the horizon distance divided by the effective radius, taken from `navigation.los.horizon` rather than recomputed here, over four decades of height and three refraction coefficients; the dip grows with height and with k; it stays below √(2h/Rₑ) everywhere and within 0.07% of it at 10 km, so the arccosine is evaluated for exactness rather than because the series fails; and at k = 0.1689 the navigator's rule runs 0.231% high at 1, 10, and 100 m alike, the constancy being the point — a rule that scales as √h can differ from the exact dip only by its coefficient
