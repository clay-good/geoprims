<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Great-circle and composite sailing (`navigation.sailing.great-circle`)

## Method

Bowditch's spherical formulas for the distance, initial course, and final course (§1307); the vertex as the great circle's point nearest a pole on the side of the track's middle; points along the track at a longitude interval; and, under a limiting latitude, the composite track of two great circles tangent to the limiting parallel joined by a run along it (§1308). The WGS 84 geodesic (Karney) is shown beside it.

## Equations

- cos D = sin L₁ sin L₂ + cos L₁ cos L₂ cos DLo.
- tan C = sin DLo / (cos L₁ tan L₂ − sin L₁ cos DLo) (atan2).
- Vertex: the great circle's normal n = p₁ × p₂; the vertex is ẑ − (ẑ·n)n normalized (cos Lv = cos L₁ |sin C|).
- Track points: tan L = tan Lv cos(λ − λv).
- Composite: cos DLovx = tan Lx / tan Lv for each end; leg distance cos d = sin Lx / sin Lv; the parallel leg DLo cos Lv.

## Symbols and units

Distances in nautical miles (minutes of arc); courses degrees true; latitudes and longitudes in degrees.

## Domain

Any two places that are neither the same nor antipodal (antipodes have no single great circle, refused with `DEGENERATE_GEOMETRY`). A composite needs both ends nearer the equator than the limit and on its side.

## Approximations

A sphere of one nautical mile per minute of arc; the WGS 84 geodesic beside it is the true shortest track and differs by up to about 0.5%.

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: The American Practical Navigator (Bowditch), Pub. No. 9, Volume I
- sourceEdition: 2024 edition
- sourceLocator: Bowditch 2024 §1307–1308: 22°S 116°E to 20°S 031°E; 28°N 122°W to 24°S 151°E; composite under 47°N from 36°57.7'N 75°42.2'W to 45°39.1'N 001°29.8'W
- independent: yes
- inputs: those places
- outputs: 4,693.5 NM, 253° T, final 289.4° T; 5,913.1 NM printed (D rounded to 98.55°; 5,913.2 NM at full precision), 247.3° T; the limit reached at 030°16.2'W (030°16.1' with unrounded tangents) and left at 018°56.9'W, where the book prints 018°57.5'W: acos(0.95397) is 17.4515°, not its 17.4651°, and its own sum would be 018°57.7'W
- tolerance: 0.05 NM and 0.05° (0.15 NM on the rounded D), 0.15' and 0.05' on the composite
- verifiedBy: golden vector v001 and `bowditch_chapter_13_examples`
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_sailings.py great-circle`: Python at 18 random pairs (spherical distance to 1e-10 relative, course to 1e-8°) with pyproj.Geod's geodesic distance and azimuth beside it (1e-6 NM, 1e-8°), and the antipodal refusal.
- `core/vectors/navigation.sailing.great-circle.jsonl`: those vectors with Bowditch's example first, run through the core on every build.

## Invariants

- `bowditch_chapter_13_examples`: the §1307 and §1308 examples. `sailing_invariants`: the great circle equals `navigation.geodesic.spherical-inverse` on a sphere of 10800/π NM to 1e-9 in distance and both courses; a composite under 45°N has three legs, the middle on 45°N, and is longer than the great circle; antipodes are refused.
