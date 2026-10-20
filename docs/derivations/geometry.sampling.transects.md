<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Parallel transects across a polygon (`geometry.sampling.transects`)

## Method

Systematic parallel line transects with a random start, the layout distance-sampling surveys use. It works on the polygon's UTM plane, as in `geometry.sampling.random-points` (step 1 of its note).

1. **Direction.** The true bearing β becomes a grid bearing by subtracting γ, the meridian convergence (the bearing of grid north from true north) at the outline's mean corner. Lines run along the unit vector **d** = (sin(β − γ), cos(β − γ)).
2. **Across.** **n** = (d_y, −d_x) points to the right of the lines. The outline's corners give the range [s_min, s_max] of s = **n**·p.
3. **Spacing and start.** The plane spacing is the ground spacing times the zone's point scale at the mean corner. One draw u from xoshiro256** (seeded as in the random-points note) puts the first line at s_min + u·d_p. Lines follow every d_p while s < s_max.
4. **Clipping.** Each line's crossings with every ring are taken by the half-open rule (an edge crosses when its ends fall on opposite sides, with a corner exactly on the line counted on its right-hand side), then ordered along **d** and paired off. A line that crosses one pair is one transect. One that crosses two or more pairs, around a hole or a bay, has parts a, b, and so on.
5. **Lengths.** Each part's length is the geodesic between its ends (Karney's inverse). The total is their sum.
6. **Zigzag.** Every other line that crosses the polygon is reversed, with its parts in reverse order. The route runs through the parts in turn, so each join goes from one part's end to the next part's start. Its length is the geodesic sum of the parts and the joins.

## Equations

- Grid bearing: α = β − γ. **d** = (sin α, cos α), **n** = (cos α, −sin α).
- Line k: s = s_min + u·d_p + k·d_p, for k = 0, 1, … while s < s_max.
- Crossing of edge ab: f = s_a / (s_a − s_b), q = a + f(b − a), position along the line t = **d**·q.

## Symbols and units

- β: bearing in degrees from true north.
- Spacing: in m on the ground.
- Seed: as in the random-points note.
- Lengths: geodesic, in m. Ends: in degrees.

## Domain

- At most 10,000 lines across the polygon, or `LIMIT_EXCEEDED`.
- The polygon spans at most 500 km and has at most 2,000 corners.
- A spacing and start that leave no line crossing the polygon give `NO_SOLUTION`.

## Approximations

Lines are straight on the UTM plane, so they are not geodesics. Over a few kilometers the two differ by far less than a field crew can walk. The spacing is exact at the polygon's center and changes with the UTM scale away from it.

## Worked example

- sourcePublisher: geoprims (the change's own scenario)
- sourceTitle: Transects 100 m apart at 90° across a 1 km square
- sourceEdition: 2026-10-20
- sourceLocator: add-sampling-and-grids spec, "Transect total length": 10 transects of 1,000 m and 10,000 m in all
- independent: no (the R dssd systematic parallel design, the spec's reference, is not reproduced here)
- inputs: a 1 km square on the zone 13 plane at its central meridian, spacing 100 m, bearing 90°, seed 4
- outputs: 10 transects of 1,000 m ± 1 mm, 10,000 m in all; the zigzag 10,900 m
- tolerance: 1 mm per transect
- verifiedBy: `transects_scenarios`
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_grid_designs.py transects`: the method above reimplemented in Python.
  - The plane comes from PROJ's transverse Mercator, and lengths from pyproj.Geod.
  - The convergence is PROJ's analytic value, checked against a geodesic estimate from points 1 m up and down grid north, so its sign is confirmed independently of the core.
  - It covers 20 cases over five polygons with bearings from 0° to 200°, lines and zigzags. Counts agree exactly, ends within 1e-9°, and lengths, totals, and routes within 1e-5 m.
- `core/vectors/geometry.sampling.transects.jsonl`: those vectors and the too-many-lines refusal, run through the core on every build.

## Invariants

- `core/crates/gp-geometry/tests/sampling.rs` `transects_scenarios`:
  - On a true 1 km square there are 10 transects of 1,000 m ± 1 mm, totaling 10,000 m ± 1 cm.
  - The zigzag runs 10,900 m, with every other line reversed.
  - A hole splits the lines through it into parts.
