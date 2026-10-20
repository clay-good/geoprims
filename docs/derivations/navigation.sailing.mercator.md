<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Mercator sailing (`navigation.sailing.mercator`)

## Method

Meridional parts on WGS 84 give the meridional difference m; the course follows from tan C = DLo / m and the distance from plane sailing, D = l sec C, switching to parallel sailing when the course is due east or west (Bowditch §1306). The meridional parts are the closed form of the series Bowditch Volume II Table 6 is computed from on WGS 84. The exact rhumb line on WGS 84 is shown beside it.

## Equations

- M = (10800/π)[atanh(sin L) − e·atanh(e sin L)], e² = f(2 − f), f = 1/298.257223563.
- m = M₂ − M₁, tan C = DLo / m, D = l sec C (or |DLo| cos L along a parallel).
- Reached: L₂ = L₁ + D cos C / 60, DLo = m tan C.

## Symbols and units

M, m, DLo, and l in minutes; C degrees true; D nautical miles.

## Domain

Latitudes short of the poles, where meridional parts are infinite (refused with `OUT_OF_DOMAIN`).

## Approximations

The course is as exact as the meridional parts. The distance l sec C counts a minute of latitude as one nautical mile everywhere, so it runs long of the exact rhumb distance on the ellipsoid by up to about 0.5%; both are shown. Bowditch's open question, whether Table 6 and the exact formula part by more than 0.1' at high latitude, is moot: the table is computed from this formula's series, and the two agree to the table's tenth (2033.45 and 2377.15 against the printed 2033.4 and 2377.1).

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: The American Practical Navigator (Bowditch), Pub. No. 9, Volume I
- sourceEdition: 2024 edition
- sourceLocator: Bowditch 2024 §1306: 32°14.7'N 66°28.9'W to 36°58.7'N 75°42.2'W
- independent: yes
- inputs: the two places
- outputs: M₁ 2033.4, M₂ 2377.1, m 343.7, DLo 553.3' W, 301.8° T; the book prints 538.7 NM from 58.18521°, but atan(553.3/343.7) is 58.1526°, which gives 538.2 NM
- tolerance: 0.05 (0.06 on M)
- verifiedBy: golden vector v001 and `bowditch_chapter_13_examples`
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_sailings.py mercator`: Python at 19 random cases in both directions (course and meridional parts to 1e-9, distance to 1e-10 relative) and the polar refusal.
- `core/vectors/navigation.sailing.mercator.jsonl`: those vectors with Bowditch's example first, run through the core on every build.

## Invariants

- `bowditch_chapter_13_examples`: the §1306 example; the exact rhumb's course agrees within 0.01° and the sphere's distance runs long by under 0.5%. `sailing_invariants`: a course and distance from the inverse lead back to the place given within 1e-9°.
