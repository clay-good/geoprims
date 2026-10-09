<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Line of sight over a ground profile (`raster.terrain.line-of-sight`)

## Method

The ground under the path is given as distance and elevation pairs from the observer to the target. Refraction bends rays toward the ground, and the standard way to model it is to keep the ray straight and flatten the Earth instead: the Earth becomes a sphere of effective radius Rₑ = R / (1 − k), with surface distances and heights unchanged. On that sphere each profile point sits at the angle s / Rₑ from the observer, the sight line is the straight chord from the observer's eye or antenna to the target's, and the clearance at a point is how far the chord passes above the ground there, measured along the local vertical. The target is in sight when every interior point has positive clearance. The point with the least clearance is the one that blocks the view, or comes closest to it. The observer height that would just see the target is found point by point: the line from the target through each ground point, carried back to the observer's vertical, and the highest of those crossings is the answer. With a frequency, each point's clearance is compared with the first Fresnel radius there, and the path passes when every point clears 60% of it.

## Equations

- Effective radius: Rₑ = R / (1 − k).
- A point s along the path and z above the sphere: (x, y) = ((Rₑ + z) sin(s/Rₑ), (Rₑ + z) cos(s/Rₑ)).
- Sight line height at s, with O and T the two antenna tops, u = (sin(s/Rₑ), cos(s/Rₑ)), and D = T − O: r = (O × D) / (u × D), height r − Rₑ, where × is the 2D cross product.
- Clearance at a point: c = (r − Rₑ) − e, with e its ground elevation.
- Observer height that grazes a point G: with D = G − T, r₀ = (T × D) / ((0, 1) × D), height r₀ − Rₑ − e₀; the answer is the largest over all points, and never below zero.
- First Fresnel radius: F₁ = √(λ d₁ d₂ / d), λ = c / f, with d₁ = s and d₂ = d − s; a point passes when c ≥ 0.6 F₁.
- The planner's form, for comparison: c ≈ h_O + (h_T − h_O) s / d − (e + d₁ d₂ / (2 Rₑ)).

## Symbols and units

s is the distance from the observer along the path, d the whole path, both in the profile's distance unit; e the ground elevation and z a height above the sphere, in the profile's elevation unit; R the Earth's radius (6,371,000 m by default), k the refraction coefficient (0.13 by default, 0.25 for radio, 0 for none), Rₑ the effective radius; f the frequency and λ the wavelength; F₁ the first Fresnel radius. Heights and distances are shown in the units the first profile point uses.

## Domain

At least three profile points with increasing distances, a path of at most 1,000 km, ground elevations between −10,000 m and 10,000 m, heights above the ground between 0 and 100 km, k between −1 and 0.9, and a radius between 100 km and 100,000 km. The first point is the observer and the last is the target; only the points between them are checked for clearance, since the masts stand on the end points.

## Approximations

The geometry is exact on the sphere: no small-angle or parabolic step is taken, and the planner's bulge form agrees with it within a centimeter over 50 km. The approximations are in the model, not the arithmetic. Refraction is a single coefficient for the whole path, and real air varies with the weather. The ground between profile points is not checked, so the answer is only as good as the sampling. The Fresnel radius uses the far-field form, which holds by orders of magnitude for any real link, and the 60% rule is a planning criterion rather than a loss figure.

## Worked example

- sourcePublisher: geoprims (computed, not published)
- sourceTitle: the ridge scenario of add-spatial-indexing-and-raster, worked in Python with the polar equation of a line (tools/vectors/gen_sightline.py)
- sourceEdition: 2026
- sourceLocator: core/vectors/raster.terrain.line-of-sight.jsonl, v001
- independent: no
- inputs: ground 300, 340, 395, 330, 310 m at 0, 4, 8, 12, 16 km; observer 2 m, target 30 m above the ground; 5.8 GHz; k = 0.13
- outputs: not visible; the ridge at 8 km (395 m) rises 78.37 m above the sight line; an observer 158.74 m up would see over it; no point clears 60% of the Fresnel zone
- tolerance: 1e-9 relative, 1 µm absolute
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-09

This is not independent in the sense the stable bar asks for: the profile was chosen here and the answer worked here, in a second formulation (the polar equation of a line rather than the Cartesian chord). No published worked example of a terrain profile with these outputs has been found yet; ITU-R P.530 gives the clearance criterion but no worked path.

## Differential tests

- `tools/vectors/gen_sightline.py`: 22 vectors from the polar equation of a line through the two antennas, a different formulation from the core's Cartesian chord, across refraction, radius, units, and frequency

## Invariants

- `core/crates/gp-raster/tests/sightline.rs` `the_needed_height_grazes_the_profile`: at the reported observer height the sight line just touches the ground, a centimeter higher sees the target, and a centimeter lower does not
- `core/crates/gp-raster/tests/sightline.rs` `more_refraction_never_lowers_clearance`: clearance rises with k
- `core/crates/gp-raster/tests/sightline.rs` `swapping_the_ends_mirrors_the_answer`: observer and target swapped give the same clearance at the mirrored point and the same counts
- `core/crates/gp-raster/tests/sightline.rs` `the_planner_bulge_agrees_on_short_paths`: the planner's straight line over ground plus bulge agrees within a centimeter over 50 km
- `core/crates/gp-raster/tests/sightline.rs` `fresnel_needs_more_room_than_sight`: every blocked point is also short of the Fresnel zone, and a lower frequency leaves no fewer points short
