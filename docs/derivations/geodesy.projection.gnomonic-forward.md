<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Gnomonic forward (`geodesy.projection.gnomonic-forward`)

## Method

The gnomonic projection makes shortest paths straight. On a sphere every great circle becomes a straight line. On the ellipsoid, Karney's gnomonic (2013, section 8) keeps every geodesic through the center exactly straight, and bends any other geodesic by only a hair near the center. That makes it the natural plane for route planning and for intersecting geodesics with plane geometry. A point's distance from the center on the map is ρ = m₁₂/M₁₂, the reduced length over the geodesic scale. Its direction is the geodesic's starting azimuth.

It shows less than a hemisphere. A point whose vertical is 90° or more from the center's is refused, and so is any point where M₁₂ is not positive, because the map runs to infinity there. The convergence and scales come from the map itself, by Richardson-extrapolated differences over about 100 m of ground. On the ellipsoid, m₁₂ and M₁₂ depend on the geodesic's azimuth as well as its length, so the radial and cross-radial images are not at right angles and the sphere's shortcut would be off by a hundredth of a degree.

## Equations

- (s, α₁, α₂, m₁₂, M₁₂) = the geodesic inverse from the center to the point
- ρ = m₁₂/M₁₂; E = FE + ρ sin α₁; N = FN + ρ cos α₁
- Refused when M₁₂ ≤ 0 or the verticals at the center and the point are 90° or more apart
- Convergence, h, and k from differences of the map, extrapolated as in Richardson's method

## Symbols and units

`lat`, `lon`, `latitude_of_origin` and `longitude_of_origin` (the center) in degrees; `false_easting` and `false_northing` in any length unit; the ellipsoid. Out come `easting` and `northing` in meters, `convergence` in degrees, and `scale_meridian` and `scale_parallel`.

## Domain

Less than a hemisphere around the center, as the spec's horizon scenario requires; the scale grows without bound toward that horizon. Ellipsoids with |f| > 0.02 are refused.

## Approximations

The easting and northing are exact to the geodesic's 15 nanometers. The convergence and scales are finite differences, good to about 1e-9, which is below the digits shown.

## Worked example

- sourcePublisher: Charles Karney, GeographicLib
- sourceTitle: GeodesicProj (the Gnomonic class)
- sourceEdition: GeographicLib 2.7
- sourceLocator: GeodesicProj -g 40 -100, an implementation independent of this one
- independent: yes
- inputs: 60° N, 30° W, center 40° N, 100° W, WGS 84
- outputs: x = 4,371,212.819 m, y = 5,140,517.234 m
- tolerance: 1e-6 m
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-24

## Differential tests

- `tools/vectors/gen_azimuthal_geodesicproj.py`: GeodesicProj's coordinates for 300 random centers, points, and ellipsoids in `core/crates/gp-geodesy/tests/data/projections_azimuthal.json`, with convergence and scales from differences of those coordinates over GeographicLib ground distances, and vectors after the published examples
- `core/crates/gp-geodesy/tests/projections.rs`: `projections_match_proj` runs all 300 forward and back
- `tools/diff/runner.mjs`: `gnomonic-forward`, 10,000 random points around twelve random centers against GeographicLib's `GeodesicProj -g`, within 1 µm

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `projections_round_trip`: forward then inverse returns the point within 1 nm, beyond what the doubles themselves hold, over a cap of 60° around the center
- `core/crates/gp-geodesy/tests/projections.rs` `projection_invariants`: three points along one geodesic through the center lie on one straight line through the center, and a point 90° from the center (the spec's horizon scenario) is refused
