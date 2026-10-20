<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Mid-latitude sailing (`navigation.sailing.mid-latitude`)

## Method

Plane sailing for l and p, parallel sailing at the mean latitude Lm = (L₁ + L₂)/2 for DLo, with no middle-latitude correction, as Bowditch advises (§1305). Both problems are solved: course and distance between two places, and the place reached on a course and distance. The exact rhumb line on WGS 84 (Karney) between the same places is shown beside it with the differences in course and distance.

## Equations

- Reached: l = D cos C, p = D sin C, L₂ = L₁ + l/60, Lm = (L₁ + L₂)/2, DLo = p sec Lm.
- Between: l = (L₂ − L₁)·60, p = DLo cos Lm, tan C = p / l, D = √(l² + p²).

## Symbols and units

Latitudes and longitudes in degrees (DMS with hemisphere letters accepted); C degrees true; D, l, p nautical miles; DLo minutes.

## Domain

Runs that do not pass a pole; a run crossing the equator should be split there, as Bowditch says.

## Approximations

The mean latitude stands for the true middle latitude, and the Earth is Bowditch's sphere, so the answer drifts from the exact rhumb line on long runs and in high latitudes; the difference is reported with every answer.

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: The American Practical Navigator (Bowditch), Pub. No. 9, Volume I
- sourceEdition: 2024 edition
- sourceLocator: Bowditch 2024 §1305: 1,253 NM on 070° from 15°17.0'N 151°37.0'E
- independent: yes
- inputs: the start, course 070°, distance 1,253 NM
- outputs: 22°25.6'N 172°21.2'E at full precision (DLo 1,244.196'); the book prints 172°21.1'E from DLo rounded to 1,244.1'
- tolerance: 0.05'
- verifiedBy: golden vector v001 and `bowditch_chapter_13_examples`
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_sailings.py mid-latitude`: Python at 19 random cases in both directions (to 1e-9 NM, 1e-9° of course, 1e-10° of longitude).
- `core/vectors/navigation.sailing.mid-latitude.jsonl`: those vectors with Bowditch's example first, run through the core on every build.

## Invariants

- `bowditch_chapter_13_examples`: the §1305 example at full precision.
