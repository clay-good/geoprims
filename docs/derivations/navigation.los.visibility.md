<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Can two points see each other? (`navigation.los.visibility`)

## Method

Two raised points can see each other exactly while their horizons overlap, so the greatest distance between them is the distance from each to its own horizon, added. That is the navigator's method and it is also the exact statement of the geometry: the sightline that just reaches is the one tangent to the Earth, and it touches at the single point both horizons share. Refraction is carried as everywhere else in this family, by replacing the Earth's radius with an effective radius R/(1 − k). With a distance given as well, the same tangent construction says whether the two are inside that range, how much of the far target is cut off below the observer's horizon, and how far the sightline clears the bulge at the midpoint.

## Equations

- Distance to the horizon from height h: d(h) = Rₑ · arccos( Rₑ / (Rₑ + h) ), with Rₑ = R/(1 − k).
- Greatest range: D = d(h₁) + d(h₂).
- Visible when the distance apart is no more than D.
- Hidden height at distance s: the height whose horizon distance is s − d(h₁), which is zero while s ≤ d(h₁) and the whole target at s = D.
- Midpoint clearance: the sightline's height above the surface at the middle, against the bulge Rₑ(1 − cos(s/2Rₑ)).

## Symbols and units

h₁ is the observer's height and h₂ the target's, in meters above the surface; s is the distance between them and D the greatest range, in meters and shown in the chosen unit; R is the Earth's radius (6,371,000 m unless given), k the refraction coefficient (0.13 unless given), and Rₑ the effective radius.

## Domain

Any two non-negative heights and any distance. The heights are above the surface, not above sea level on land, and the surface is a sphere: there is no terrain in this model at all. Every answer carries `TERRAIN_NOT_CONSIDERED` for that reason. A refraction coefficient of 1 or more would make the effective radius infinite or negative and is refused.

## Approximations

The horizon distance is the exact arc, an arccosine rather than the √(2hRₑ) form the rules of thumb use, so the range is exact for the model at every height. What is approximate is the model: a sphere, and one refraction coefficient standing for the whole atmosphere. Over water an inversion can lift a target far past the range given; over land the terrain decides long before the curve does.

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: The American Practical Navigator (Bowditch), NGA Pub. 9
- sourceEdition: 2024 edition, Volume II
- sourceLocator: Table 12, distance of the horizon, used the way the text directs — the range at which two objects are visible to each other is the sum of their two horizon distances
- independent: yes
- inputs: 115 pairs of heights drawn from the table's own printed rows, at k = 0.1689
- outputs: the greatest range, against the two printed distances added
- tolerance: 0.1 nautical mile from the table's rounding, plus 0.231% of the sum, which is how far its square-root rule sits above the exact horizon
- verifiedBy: golden vector v023, run by the core on every build
- verifiedOn: 2026-09-23

The check does not re-derive anything: it takes distances Bowditch prints and combines them the way Bowditch says to. Table 12 gives the horizon distance for 126 heights, and the greatest range between two objects is those two distances added. Over 115 pairs the worst disagreement is 0.2364 nmi, on 700 ft against 700 ft, where the table prints 31.0 nmi twice and the tool gives 61.7636. Both parts of that are known and neither is slack: each printed cell is rounded to a tenth, and the table is the rule 1.17 √h, which runs 0.231% above the exact horizon — the same 0.231% that the dip tool's note pins down from the same pair of Bowditch rules. At 62 nmi that is 0.14 nmi, and the rounding is up to 0.1 nmi more.

The 640 ft row is skipped: the table misprints it as 29.5 nmi where its own rule gives 29.599, which `horizon_parity.rs` already pins as a misprint rather than a disagreement.

## Differential tests

- `core/crates/gp-navigation/tests/horizon_parity.rs` `visibility_is_two_horizons_of_bowditch_table_12`: 115 pairs of printed heights, the greatest range against the two printed distances added
- `tools/vectors/gen_los.py`: 22 vectors across observer and target heights and distances inside and outside the range
- `core/vectors/navigation.los.visibility.jsonl`: 23 vectors, the last from Table 12's own rows

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `visibility_invariants`: the greatest range is the two horizon distances added, both taken from `navigation.los.horizon` rather than recomputed, and the reported observer horizon is that tool's answer exactly, at four height pairs and three refraction coefficients; `visible` agrees with the distance being within the range; the hidden height is zero out to the observer's own horizon, rises from there, and is the whole target at the greatest range; and the hidden height at a given fraction of the greatest range barely moves with k, holding to a part in 100,000 between no refraction and 0.1689 — not an identity, since the two heights stay fixed while the effective radius changes, but close enough to catch a coefficient applied in the wrong place
