<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Exact transverse Mercator forward (`geodesy.projection.tm-exact-forward`)

## Method

This is the same transverse Mercator as the series tool, conformal and true to scale along the central meridian, but computed exactly. It uses Lee's (1976) formulation in Jacobi elliptic functions, with the implementation ported line by line from GeographicLib's TransverseMercatorExact (Karney 2011, section 4). First a point's latitude and longitude are mapped to Thompson's transverse Mercator coordinates (u, v) by inverting an elliptic-function relation with Newton's method, from carefully chosen starting points. Then (u, v) go to the easting and northing through the elliptic integral of the second kind. Carlson's symmetric integrals and Bulirsch's algorithm for sn, cn, and dn supply the special functions.

The result holds to double precision anywhere on the ellipsoid. That matters far from the central meridian, where the sixth-order series drifts by meters: 15 m near 80° of longitude. Points more than 90° from the central meridian fold onto the far side of the grid (GeographicLib's standard domain). On the equator at 90(1 − e)° from the central meridian, 82.636° on WGS 84, the map has a branch point where it folds and stops being conformal, though its scale stays finite.

## Equations

- (τ′, λ) from the point, τ′ the tangent of the conformal latitude
- Solve ζ(u, v) = (τ′, λ) by Newton's method, where ζ is Lee's 54.17 in sn, cn, dn of u (parameter e²) and of v (parameter 1 − e²)
- ξ = E(u) − e² sn cn dn(u)/d, η = v − E(v) + (1 − e²) sn cn dn(v)/d, with d = e² cn²(u) + (1 − e²) cn²(v) (Lee 55.4)
- x = k₀aη; y = k₀aξ; E = FE + x; N = FN + y − y(φ₀)
- Convergence γ = atan2((1 − e²) sn u sn v cn v, cn u dn u dn v) and the scale from Lee 55.12 and 55.13

## Symbols and units

`lat`, `lon` in degrees; `longitude_of_origin` (required), `latitude_of_origin` (0 if not given), `scale_factor` (1 if not given), `false_easting` and `false_northing` in any length unit, and an oblate ellipsoid. Out come `easting` and `northing` in meters, `convergence` in degrees, and the two scales, equal because the projection is conformal.

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
- inputs: 30° N, 75° E, central meridian 0°, k₀ 0.9996, WGS 84
- outputs: E = 7,707,953.714163 m, N = 7,322,160.469546 m, convergence 62.089230526°, scale 1.818307886
- tolerance: 1e-5 m
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
