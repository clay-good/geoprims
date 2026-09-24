<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Orthographic forward (`geodesy.projection.orthographic-forward`)

## Method

The orthographic shows the Earth as a camera infinitely far above the center would: each point drops straight down, along the center's vertical, onto the plane touching the ellipsoid there. Only the half facing the camera has an image. Near the center shapes look natural, and toward the rim they foreshorten until the rim itself is seen edge-on.

This is EPSG's ellipsoidal method 9840, following IOGP Guidance Note 7-2. On a sphere the formulas collapse to the familiar ones. On the ellipsoid, the prime-vertical radius ν at the point and at the center, and a term in e², keep the projection exactly orthogonal to the center's vertical.

## Equations

- ν = a/√(1 − e² sin² φ), ν₀ the same at the center φ₀; Δλ = λ − λ₀
- E = FE + ν cos φ sin Δλ
- N = FN + ν (sin φ cos φ₀ − cos φ sin φ₀ cos Δλ) + e² (ν₀ sin φ₀ − ν sin φ) cos φ₀
- ∂(E, N)/∂φ = ρ (−sin φ sin Δλ, cos φ₀ cos φ + sin φ₀ sin φ cos Δλ), with ρ the meridian radius; ∂(E, N)/∂λ = ν cos φ (cos Δλ, sin φ₀ sin Δλ)
- Convergence = −atan2 of the first column; h = its length / ρ; k = the second's length / (ν cos φ)

## Symbols and units

`lat`, `lon`, `latitude_of_origin` and `longitude_of_origin` (the center) in degrees; `false_easting` and `false_northing` in any length unit; the ellipsoid. Out come `easting` and `northing` in meters, `convergence` in degrees, and `scale_meridian` and `scale_parallel`.

## Domain

The near side: a point whose vertical is 90° or more from the center's is refused, since it would land on top of a point on the near side.

## Approximations

None: the forward formulas and their derivatives are exact.

## Worked example

- sourcePublisher: PROJ contributors
- sourceTitle: PROJ, the ortho projection (EPSG method 9840)
- sourceEdition: PROJ 9.3 (pyproj 3.6.1) and PROJ 9.9 (the proj program)
- sourceLocator: +proj=ortho +lat_0=55 +lon_0=5 +ellps=WGS84
- independent: yes
- inputs: 50° N, 9° E
- outputs: E = 286,550.1136 m, N = −547,480.6206 m
- tolerance: 1e-6 m
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-24

## Differential tests

- `tools/vectors/gen_projections_proj.py`: 300 random centers and points on the near side, on five ellipsoids, from PROJ's ortho, in `core/crates/gp-geodesy/tests/data/projections_proj.json`, and vectors after the published example
- `core/crates/gp-geodesy/tests/projections.rs`: `projections_match_proj` runs all 300 forward and back
- `tools/diff/runner.mjs`: `orthographic-forward`, 10,000 random points within 80° of twelve random centers against PROJ's `proj +proj=ortho`, within 1 µm

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `projections_round_trip`: forward then inverse returns the point within 1 nm, beyond what the doubles themselves hold, over a cap of 60° around the center
- `core/crates/gp-geodesy/tests/projections.rs` `projection_invariants`: the center lands on the falsings with a scale of exactly 1 each way, and a point on the far side is refused
