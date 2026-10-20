<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Systematic grid sample points (`geometry.sampling.systematic-points`)

## Method

A lattice laid over the polygon on its UTM zone's transverse Mercator plane, built the same way as for `geometry.sampling.random-points` (step 1 of its note), keeping the points inside the outline and outside the holes. Every step is fixed so a seed reproduces the grid exactly.

1. **Spacing.** The ground spacing d is multiplied by the zone's point scale k at the outline's mean corner. This makes neighbors d apart on the ground, not on the plane.
2. **Frame.** The origin is the lower-left corner of the outline's bounding box on the plane. A rotation θ, clockwise like a bearing, turns the grid about that origin, and the outline is turned back by θ to find its extent in the grid's frame.
3. **Start.** A random start draws u then v from xoshiro256** (seeded as in the random-points note), moving the lattice by (u·d, v·r), where r is the row spacing. A corner start uses (0, 0) and ignores the seed.
4. **Lattice.**
   - Square: rows and columns d apart.
   - Triangular: rows r = d·√3/2 apart, with odd rows shifted d/2, so every point has six neighbors at d.
   - Rows run south to north and points west to east in the grid's frame, starting one column early on a shifted row when the shift would leave a gap at the west edge.
5. **Keep.** A point is kept when an even-odd ray test over all rings puts it inside the polygon. It is then carried back to latitude and longitude.

## Equations

- Plane spacing d_p = d · k(φ₀, λ₀). Row spacing r = d_p (square) or d_p·√3/2 (triangular).
- Point (i, j): x = x₀ + u·d_p + s_j + i·d_p, y = y₀ + v·r + j·r, with s_j = d_p/2 on odd rows of a triangular grid. Then turned by θ about the origin.
- Area per point: d² (square) or d²·√3/2 (triangular), on the ground.
- Expected count, checked before any point is made: plane area / (d_p · r).

## Symbols and units

- d: spacing in m (any length unit is converted).
- θ: rotation in degrees, clockwise.
- Seed: as in the random-points note.
- Points: in degrees. Area per point: in m².

## Domain

- At most 100,000 points expected, or `LIMIT_EXCEEDED` names the count. For example, a 5 m grid over 100 km² is about 4,000,000.
- At most 10,000,000 grid positions over the bounding box.
- The polygon spans at most 500 km and has at most 2,000 corners.
- A spacing that leaves no point inside is `NO_SOLUTION`.

## Approximations

The spacing is exact at the polygon's center. Elsewhere it changes with the UTM scale: by under 1 mm over 1 km near the center of a small site, more for a site far from its zone's central meridian and many kilometers across.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: A 50 m triangular grid over a 340 m × 333 m field
- sourceEdition: 2026-10-20
- sourceLocator: Each interior point's six nearest neighbors measured by geodesic inverse (GeographicLib) at 50 m
- independent: no (QGIS regular-points output, the spec's reference, is not reproduced here)
- inputs: the field, d = 50 m, triangular, seed 3
- outputs: six neighbors at 50 m for every interior point
- tolerance: 1 mm
- verifiedBy: `systematic_points_scenarios`
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_grid_designs.py systematic`: the method above reimplemented in Python, with PROJ's transverse Mercator and its scale factor (pyproj). It covers 20 cases over five polygons with square and triangular grids, random and corner starts, and rotations of 0° to 100°. Counts agree exactly, and the first 12 points within 1e-9°.
- `core/vectors/geometry.sampling.systematic-points.jsonl`: those vectors and the too-many-points refusal, run through the core on every build.

## Invariants

- `core/crates/gp-geometry/tests/sampling.rs` `systematic_points_scenarios`:
  - Triangular interior points have six neighbors, and square ones four, at 50 m ± 1 mm on the ground.
  - A 30° grid keeps its spacing.
  - A corner start ignores the seed, while a random start follows it and repeats for the same seed.
  - A 5 m grid over 100 km² is refused, stating about 4,000,000 points against 100,000.
