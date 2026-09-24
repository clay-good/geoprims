<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Gnomonic inverse (`geodesy.projection.gnomonic-inverse`)

## Method

This undoes the gnomonic, as GeographicLib's Gnomonic::Reverse does. The direction from the center gives the geodesic's starting azimuth at once. The distance along it is found by Newton's method on ρ(s) = m₁₂/M₁₂, whose slope is 1/M₁₂². Far from the center it solves 1/ρ instead, which is better conditioned there. A point that does not settle in ten steps is refused.

## Equations

- α₁ = atan2(E − FE, N − FN); ρ = √((E − FE)² + (N − FN)²)
- Start s = a·atan(ρ/a); step s ← s − (m₁₂ − ρM₁₂)·M₁₂ (or s ← s − (m₁₂/ρ − M₁₂)·m₁₂ when ρ > a) until the step is below 0.01·√ε·a
- (φ, λ) = the geodesic direct from the center at the final s

## Symbols and units

`easting` and `northing` in any length unit, with the same center, falsings, and ellipsoid as the forward direction. Out come `lat` and `lon` in degrees.

## Domain

Any easting and northing; far from the center the point lies near the horizon, where a meter on the grid is little on the ground.

## Approximations

Newton's method stops within about 1e-10 m of distance; the result is then as exact as the geodesic.

## Worked example

- sourcePublisher: Charles Karney, GeographicLib
- sourceTitle: GeodesicProj (the Gnomonic class)
- sourceEdition: GeographicLib 2.7
- sourceLocator: GeodesicProj -g 40 -100 -r
- independent: yes
- inputs: x = 4,371,212.818782 m, y = 5,140,517.234103 m, center 40° N, 100° W
- outputs: 60° N, 30° W
- tolerance: 1e-10°
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-24

## Differential tests

- `tools/vectors/gen_azimuthal_geodesicproj.py`: GeodesicProj's coordinates for 300 random centers, points, and ellipsoids in `core/crates/gp-geodesy/tests/data/projections_azimuthal.json`, with convergence and scales from differences of those coordinates over GeographicLib ground distances, and vectors after the published examples
- `core/crates/gp-geodesy/tests/projections.rs`: `projections_match_proj` runs all 300 forward and back
- `tools/diff/runner.mjs`: `gnomonic-forward`, 10,000 random points around twelve random centers against GeographicLib's `GeodesicProj -g`, within 1 µm; the inverse is checked by returning each fixture point from GeodesicProj's coordinates

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `projections_round_trip`: forward then inverse returns the point within 1 nm, beyond what the doubles themselves hold, over a cap of 60° around the center
- `core/crates/gp-geodesy/tests/projections.rs` `projection_invariants`: three points along one geodesic through the center lie on one straight line through the center, and a point 90° from the center (the spec's horizon scenario) is refused
