<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Orthographic inverse (`geodesy.projection.orthographic-inverse`)

## Method

The orthographic shows the Earth as a camera infinitely far above the center would: each point drops straight down, along the center's vertical, onto the plane touching the ellipsoid there. Only the half facing the camera has an image. Near the center shapes look natural, and toward the rim they foreshorten until the rim itself is seen edge-on.

This is EPSG's ellipsoidal method 9840, following IOGP Guidance Note 7-2. On a sphere the formulas collapse to the familiar ones. On the ellipsoid, the prime-vertical radius ν at the point and at the center, and a term in e², keep the projection exactly orthogonal to the center's vertical.

This tool is the inverse. EPSG gives no closed form for it, so the point is found by Newton's method in latitude and longitude, starting from the sphere's answer and using the exact Jacobian of the forward map.

## Equations

- First guess: the sphere of radius ν₀, c = asin(ρ′/ν₀) with ρ′ the distance from the falsings
- Newton's method on (φ, λ) with the Jacobian above, until one step after the update falls under 1e-12 radians
- Refused when ρ′ > ν₀ (off the disk), when the Jacobian vanishes (the rim), or when the answer lies on the far side

## Symbols and units

`easting` and `northing` in any length unit, with the same center, falsings, and ellipsoid as the forward direction. Out come `lat` and `lon` in degrees.

## Domain

The disk the near side covers, about one prime-vertical radius across from the center; a point outside it, or on its rim where the map folds, is refused.

## Approximations

None beyond Newton's method, which ends at the rounding of a double.

## Worked example

- sourcePublisher: PROJ contributors
- sourceTitle: PROJ, the ortho projection (EPSG method 9840)
- sourceEdition: PROJ 9.3 (pyproj 3.6.1) and PROJ 9.9 (the proj program)
- sourceLocator: +proj=ortho +lat_0=55 +lon_0=5 +ellps=WGS84
- independent: yes
- inputs: E = 286,550.1136 m, N = −547,480.6206 m
- outputs: 50° N, 9° E
- tolerance: 1e-10°
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-24

## Differential tests

- `tools/vectors/gen_projections_proj.py`: 300 random centers and points on the near side, on five ellipsoids, from PROJ's ortho, in `core/crates/gp-geodesy/tests/data/projections_proj.json`, and vectors after the published example
- `core/crates/gp-geodesy/tests/projections.rs`: `projections_match_proj` runs all 300 forward and back
- `tools/diff/runner.mjs`: `orthographic-forward`, 10,000 random points within 80° of twelve random centers against PROJ's `proj +proj=ortho`, within 1 µm

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `projections_round_trip`: forward then inverse returns the point within 1 nm, beyond what the doubles themselves hold, over a cap of 60° around the center
- `core/crates/gp-geodesy/tests/projections.rs` `projection_invariants`: the center lands on the falsings with a scale of exactly 1 each way, and a point on the far side is refused
