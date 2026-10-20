<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Random sample points in a polygon (`geometry.sampling.random-points`)

## Method

A simple random sample by triangulation, not by rejection, so the number of random draws never depends on the polygon's shape. Every step is fixed below, so the same seed gives the same points on the website, the MCP server, and any faithful reimplementation.

1. **Plane.** The zone is the UTM zone of the outline's mean corner, with longitudes unwrapped about the first corner. Every corner goes to that zone's transverse Mercator plane (WGS 84, k₀ = 0.9996) with no false easting or northing, so the plane stays continuous across the equator. The outline is made counterclockwise and the holes clockwise.
2. **Holes.** Holes are taken in order of their largest x, largest first, with ties going to the earlier hole. Each hole's corner with the largest x (the first such corner) is joined to the nearest outline corner, ties going to the lower index. The join must not cross or touch any side of the outline as built so far, or of the holes still to come. The joined ring runs: outline up to that corner, around the hole from the joined corner back to it, then the outline corner again.
3. **Ears.** Starting at position 0, the tool looks for an ear at position i. It takes the corner before i (a), the corner at i (b), and the corner after i (c). These form an ear when the turn a, b, c is strictly counterclockwise and no reflex corner of the ring, other than copies of a, b, or c, lies inside or on the triangle. A found ear is emitted as (a, b, c) and its corner removed, and the search stays at the same position. If not, i moves on by one. A full pass with no ear is refused. The last three corners make the last triangle.
4. **Draws.** The generator is xoshiro256** with its state filled by four splitmix64 outputs from the seed, and doubles are (x >> 11) × 2⁻⁵³. Each point takes exactly two draws:
   - u = draw × total area. The triangle is the first whose cumulative area exceeds u.
   - r₁ = (u − area before that triangle) / that triangle's area, which is uniform in [0, 1) and independent of the choice.
   - r₂ is the second draw.
5. **Point.** P = (1 − √r₁)A + √r₁(1 − r₂)B + √r₁r₂C (Osada et al. 2002 §4.2). The point is then carried back to latitude and longitude.

Osada et al. draw a third number to choose the triangle. Reusing the first draw, rescaled within the chosen triangle's share, keeps the stream at two draws per point, as the design requires, without changing the distribution.

## Equations

- splitmix64: x += 0x9e3779b97f4a7c15; z = (x ⊕ x≫30)·0xbf58476d1ce4e5b9; z = (z ⊕ z≫27)·0x94d049bb133111eb; return z ⊕ z≫31.
- xoshiro256**: result = rotl(s₁·5, 7)·9; t = s₁≪17; s₂ ⊕= s₀; s₃ ⊕= s₁; s₁ ⊕= s₂; s₀ ⊕= s₃; s₂ ⊕= t; s₃ = rotl(s₃, 45).
- Triangle area: ½[(B − A) × (C − A)]. Point: as in step 5.
- Area per point: geodesic polygon area (Karney 2013), holes out, divided by N.

## Symbols and units

- Polygon: corners in decimal degrees, with ring 0 the outline and 1, 2, … the holes.
- N: number of points, 1 to 100,000.
- Seed: a whole number from 0 to 2⁶⁴ − 1, default 1, entered as text or a JSON integer.
- Points: in degrees. Areas: in m².

## Domain

- The polygon spans at most 500 km on the plane and has at most 2,000 corners in all.
- No ring may cross or touch itself or another ring.
- A larger polygon is refused with `LIMIT_EXCEEDED`. A crossing ring, or one the ears cannot cut, is refused with `DEGENERATE_GEOMETRY`.

## Approximations

The points are uniform by area on the UTM plane. Within a zone the scale runs from 0.9996 on the central meridian to about 1.0010 at the zone's edge, so the density on the ground varies by up to about 0.3%. A polygon reaching past its zone sees more. The sides are straight on the plane rather than geodesics. Over a 500 m side the difference is micrometers.

## Worked example

- sourcePublisher: Blackman, D., and Vigna, S. (reference code)
- sourceTitle: xoshiro256starstar.c and splitmix64.c, public domain
- sourceEdition: retrieved 2026-10-20 from prng.di.unimi.it
- sourceLocator: The generator step and splitmix64 seeding, compiled with clang, give for seed 42 the outputs 1546998764402558742, 6990951692964543102, …, and for seed 0 the outputs 11091344671253066420, …
- independent: yes for the stream; the point layout has no published example
- inputs: seeds 0, 42, and 2⁶⁴ − 1
- outputs: the first 10 outputs for seeds 0 and 42, and the first for 2⁶⁴ − 1, pinned in `gp-base` `rng::tests::reference_streams`
- tolerance: exact
- verifiedBy: `reference_streams` and `doubles_take_the_top_53_bits`
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_random_points.py`: the method above reimplemented in Python with the same stream. PROJ's transverse Mercator (through pyproj) stands in for the core's, and pyproj.Geod gives the area. It covers 20 cases over five polygons (holes, concave, the southern hemisphere, the equator) and a range of seeds.
  - Every point agrees within 1e-9° (0.1 mm), and the area within 1e-9 relative.
  - The spec asks for byte parity, but two transverse Mercator implementations differ in the last bits, so the vectors carry 1e-9°.
- `core/vectors/geometry.sampling.random-points.jsonl`: those vectors plus the over-500 km and bad-seed refusals, run through the core on every build.

## Invariants

- `core/crates/gp-geometry/tests/sampling.rs` `random_points_invariants`:
  - 10,000 points in an L-shaped field with a hole all land inside the outline and outside the hole;
  - the same seed, given as text or a number, gives identical points, and another seed does not;
  - 100,000 points split between two equal halves of a square within 49,000 to 51,000.
- `random_points_refusals`: over 500 km, a negative seed, and the largest seed.
