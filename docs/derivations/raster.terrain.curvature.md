<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Terrain curvature (`raster.terrain.curvature`)

## Method

Zevenbergen and Thorne (1987) fit a surface through all nine elevations of a 3 x 3 window, the partial quartic z = Ax²y² + Bx²y + Cxy² + Dx² + Ey² + Fxy + Gx + Hy + I, whose nine coefficients the nine elevations fix exactly. At the center cell only five of them matter: G and H are the slope east and north, and D, E, and F are the second derivatives. Total curvature is how the surface bends overall; profile curvature is how it bends down the slope, along the gradient, which is whether the slope steepens or flattens downhill; plan curvature is how it bends across the slope, which is whether the contours bow out over a spur or curl into a hollow. Here every measure is positive where the ground is convex, and each is reported per 100 m.

## Equations

With z1 … z9 the window row by row from the north-west (z2 north, z4 west, z5 the center), dx and dy the cell sizes east and north:

- D = ((z4 + z6) / 2 − z5) / dx², E = ((z2 + z8) / 2 − z5) / dy², F = (z3 + z7 − z1 − z9) / (4 dx dy)
- G = (z6 − z4) / (2 dx), H = (z2 − z8) / (2 dy)
- Total curvature = −2 (D + E) × 100
- Profile curvature = −2 (D G² + E H² + F G H) / (G² + H²) × 100
- Plan curvature = −2 (D H² + E G² − F G H) / (G² + H²) × 100
- Profile + plan = total, which follows from the two numerators summing to (D + E)(G² + H²).

## Symbols and units

Elevations and cell sizes in meters (a cell size in degrees is converted with the WGS 84 meridian and prime-vertical radii at the latitude given, as the slope tool does). D, E, and F are per meter; G and H are rise over run. Each curvature is per meter times 100, so a value of 1 means the slope changes by 1% for every meter moved.

## Domain

Any finite elevations and positive cell sizes. Profile and plan curvature need a downhill direction, so on a window whose center has no gradient (G = H = 0) they are not given and the flat-cell warning is raised; total curvature is always given.

## Approximations

None in the arithmetic: the result is exact for the partial quartic, and so exact for any surface that is a quadratic or a partial quartic across the window. The approximation is in taking that surface for the ground: curvatures are second differences, so noise in a single elevation moves them a lot, and they shrink as the cell size grows. These are the simplified forms Zevenbergen and Thorne give, without the (1 + G² + H²)^(3/2) normalization of the full differential-geometric curvatures, which differs from them on steep ground; other programs differ in sign and in the factor of 100.

## Worked example

- sourcePublisher: geoprims (computed, not published)
- sourceTitle: a partial quartic sampled on a 10 m window, its curvatures worked from the polynomial's own derivatives (tools/vectors/gen_curvature.py)
- sourceEdition: 2026
- sourceLocator: core/vectors/raster.terrain.curvature.jsonl, v001
- independent: no
- inputs: z = 103 + 0.02x + 0.25y + 0.003x² − 0.002y² + 0.0005xy sampled at 10 m: rows 105.35, 105.3, 105.85 / 103.1, 103.0, 103.5 / 100.45, 100.3, 100.75
- outputs: total −0.2, profile 0.3857, plan −0.5857: the slope steepens downhill and the contours curl into a hollow
- tolerance: 1e-9
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-09

The expected values come from calculus on the surface the window was sampled from, never from the window, so a wrong index or sign in the core fails them. It is not a published example: Zevenbergen and Thorne print the method without a numeric window, and no published worked case has been found yet.

## Differential tests

- `tools/vectors/gen_curvature.py`: 23 vectors from the analytic derivatives of sampled surfaces, including quartic terms that change the grid but not the center, random surfaces, and one-arc-second cells at three latitudes

## Invariants

- `core/crates/gp-raster/tests/curvature.rs` `turning_or_mirroring_the_ground_changes_nothing`: every quarter turn and an east-west mirror give the same three curvatures
- `core/crates/gp-raster/tests/curvature.rs` `profile_and_plan_add_up_to_the_total`: profile + plan = total on 40 windows
- `core/crates/gp-raster/tests/curvature.rs` `tilting_the_ground_leaves_the_total_alone`: adding a plane does not change total curvature
- `core/crates/gp-raster/tests/curvature.rs` `heights_scale_and_flip_the_answer`: tripling the elevations triples each curvature, negating them negates it, and doubling the cell size quarters it
- `core/crates/gp-raster/tests/curvature.rs` `a_flat_center_has_no_downhill_direction`: a summit is convex, with no profile or plan curvature and the flat-cell warning
