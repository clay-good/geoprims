<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Exact transverse Mercator inverse (`geodesy.projection.tm-exact-inverse`)

## Method

This is the same transverse Mercator as the series tool, conformal and true to scale along the central meridian, but computed exactly. It uses Lee's (1976) formulation in Jacobi elliptic functions, with the implementation ported line by line from GeographicLib's TransverseMercatorExact (Karney 2011, section 4). First a point's latitude and longitude are mapped to Thompson's transverse Mercator coordinates (u, v) by inverting an elliptic-function relation with Newton's method, from carefully chosen starting points. Then (u, v) go to the easting and northing through the elliptic integral of the second kind. Carlson's symmetric integrals and Bulirsch's algorithm for sn, cn, and dn supply the special functions.

The result holds to double precision anywhere on the ellipsoid. That matters far from the central meridian, where the sixth-order series drifts by meters: 15 m near 80° of longitude. Points more than 90° from the central meridian fold onto the far side of the grid (GeographicLib's standard domain). On the equator at 90(1 − e)° from the central meridian, 82.636° on WGS 84, the map has a branch point where it folds and stops being conformal, though its scale stays finite.

This tool is the inverse, from an easting and northing back to latitude and longitude with the same parameters.

## Equations

- ξ = (N − FN + y(φ₀))/(k₀a); η = (E − FE)/(k₀a)
- Solve σ(u, v) = (ξ, η) by Newton's method from GeographicLib's starting points (a pole, a cubic near the branch point, or the scaled identity)
- (τ′, λ) = ζ(u, v); τ from τ′ by Newton's method; φ = atan τ; λ plus the central meridian

## Symbols and units

`easting` and `northing` in any length unit, with the same parameters as the forward direction. Out come `lat` and `lon` in degrees.

## Domain

The whole ellipsoid, in GeographicLib's standard domain. A sphere (flattening 0) and flattening over 0.1 are refused; the series tool is exact on a sphere.

## Approximations

None beyond double precision. The Newton iterations stop as GeographicLib's do, one step after the update falls below machine precision.

## Worked example

- sourcePublisher: Charles Karney, GeographicLib
- sourceTitle: TransverseMercatorProj (the TransverseMercatorExact class)
- sourceEdition: GeographicLib 2.7
- sourceLocator: TransverseMercatorProj -l 0 -k 0.9996, its default exact mode
- independent: yes
- inputs: E = 7,707,953.714163 m, N = 7,322,160.469546 m
- outputs: 30° N, 75° E
- tolerance: 1e-10°
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-24

GeographicLib is the implementation this tool is ported from, but it is compiled from the author's C++ source and run as a separate program, so it checks the port rather than repeating it. At 75° from the central meridian the series would already be meters off.

## Differential tests

- `tools/vectors/gen_tm_exact_geographiclib.py`: 300 random grids and points anywhere on the globe, on five ellipsoids, from TransverseMercatorProj's exact mode, in `core/crates/gp-geodesy/tests/data/projections_tm_exact.json`, and vectors after the published examples
- `core/crates/gp-geodesy/tests/projections.rs`: `projections_match_proj` runs all 300 forward and back, the grid within 15 nm, the convergence within 1e-13°, and the scale within 3e-15 of itself
- `tools/diff/runner.mjs`: `tm-exact-forward`, 10,000 random points on twelve random grids anywhere on the globe against TransverseMercatorProj, within 1 µm

A separate check at 20,000 points over the whole WGS 84 globe, poles, equator, and the branch point included, agreed within 5.6e-8 m, 5e-11° of convergence, and 7e-15 of scale. It also showed that TransverseMercatorProj's `-t` flag selects the extended domain, which folds the far side differently, so the default mode is the reference.

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `projections_round_trip`: forward then inverse returns the point within 1 nm, beyond what the doubles themselves hold, up to 60° of longitude from the central meridian, well past the series' reach
