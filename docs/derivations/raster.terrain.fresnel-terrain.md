<!-- Derivation note (trust/correctness-program layer A). -->
# Radio link Fresnel clearance over terrain (`raster.terrain.fresnel-terrain`)

## Method

A radio link needs more than a clear sight line: the first Fresnel zone, an ellipsoid around the line, must also be mostly free of ground. This operation runs the same code as `raster.terrain.line-of-sight` on the ground profile between two antennas and reports, first, whether every profile point clears 60% of the first Fresnel radius, the free-space screen in ITU-R P.530. It requires the frequency and defaults to radio refraction, k = 0.25 (a 4/3 Earth).

## Equations

- Effective radius Re = R / (1 − k); each profile point sits at the angle s / Re from the first antenna
- Sight line: the straight chord between the two antennas; clearance c = chord height − ground, along the local vertical
- First Fresnel radius F1 = √(λ d1 d2 / d), λ = c₀ / f
- Clearance share = c / F1; a point passes when it is at least 0.6

## Symbols and units

Distances s, d1, d2, and d (the whole path) in meters; ground elevations and antenna heights above ground in meters; frequency f in hertz, with c₀ = 299,792,458 m/s; R the Earth radius, 6,371,000 m by default; k the refraction coefficient.

## Domain

At least three profile points with increasing distances, a path up to 1,000 km, ground from −10,000 m to 10,000 m, antenna heights from 0 to 100 km, and a frequency above zero. Without a frequency the operation refuses with INVALID_INPUT.

## Approximations

Only the profile points are checked; the ground between them is not. Refraction enters only through k. The 60% screen is a planning check, not a diffraction-loss calculation.

## Worked example

- sourcePublisher: International Telecommunication Union (the Fresnel radius and 60% screen); the geometry worked independently in Python
- sourceTitle: ITU-R P.530, propagation data and prediction methods for terrestrial line-of-sight systems
- sourceEdition: P.530-19 (09/2025)
- sourceLocator: sections 2.2.1 and 2.2.2; the sight line written as the polar equation of a line through two points, not the core's Cartesian intersection
- independent: no
- inputs: a 12 km, 5.8 GHz hop with antennas 38 m and 32 m above ground over a 236 m hill at 6 km
- outputs: in sight with 4.38 m to spare, but the hill fills the Fresnel zone to 35.2%, so one point falls short of 60%: golden vector v001
- tolerance: 1e-6 percent and 1e-6 m
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-14

## Differential tests

- `tools/vectors/gen_fresnel_terrain.py`: 22 paths from 2 km to 40 km at 0.9 GHz to 23 GHz, at the default k and at 0.13 and 0, with the sight line from its polar equation rather than the core's chord intersection.
- `core/vectors/raster.terrain.fresnel-terrain.jsonl`: those vectors and refusals for a missing and a zero frequency.
- `core/crates/gp-raster/tests/sightline.rs` `fresnel_terrain_matches_line_of_sight_at_radio_k` requires the line-of-sight tool's answer at k = 0.25.

## Invariants

- A point that fails the sight line also fails the 60% screen.
- A higher frequency never needs more clearance, since the Fresnel radius shrinks.
- The worst point's share is the least of every point's share.
