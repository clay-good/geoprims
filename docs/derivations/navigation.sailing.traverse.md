<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Traverse sailing (`navigation.sailing.traverse`)

## Method

Each leg is solved as a plane sailing, the differences of latitude and departures are summed, and the sums are turned into one course and distance made good (Bowditch §1303). A current can be entered as an extra leg.

## Equations

- Σl = Σ Dᵢ cos Cᵢ, Σp = Σ Dᵢ sin Cᵢ.
- tan C = Σp / Σl (atan2), D = √(Σl² + Σp²).

## Symbols and units

Courses in degrees true; distances, l, and p in nautical miles.

## Domain

1 to 1,000 legs; no negative distance.

## Approximations

Plane sailing on every leg. Bowditch works the example with traverse tables rounded to 0.1 NM per leg; the tool sums exactly, so it can differ from a table solution by a few tenths of a mile.

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: The American Practical Navigator (Bowditch), Pub. No. 9, Volume I
- sourceEdition: 2024 edition
- sourceLocator: Bowditch 2024 §1303: Five legs: 158° 15.5, 135° 33.7, 259° 16.1, 293° 39.0, 169° 40.4 NM
- independent: yes
- inputs: the five legs
- outputs: 192.3° T, 67.3 NM printed (sums 65.8 S, 14.4 W from rounded table values); exact sums give 192.3° and 67.24 NM
- tolerance: 0.05° and 0.1 NM
- verifiedBy: golden vector v001 and `bowditch_chapter_13_examples`
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_sailings.py traverse`: Python sums over 19 random traverses of 1 to 8 legs, exact to 1e-9.
- `core/vectors/navigation.sailing.traverse.jsonl`: those vectors with Bowditch's example first, run through the core on every build.

## Invariants

- `bowditch_chapter_13_examples`: the §1303 example within the table rounding.
