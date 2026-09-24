<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Azimuthal Equidistant inverse (`geodesy.projection.azimuthal-equidistant-inverse`)

## Method

This undoes the azimuthal equidistant: an easting and northing are a distance and a direction from the center, and the point is where the geodesic leaving the center in that direction ends after that distance. It is the geodesic direct problem, solved exactly (Karney 2013).

## Equations

- ρ = √((E − FE)² + (N − FN)²); α₁ = atan2(E − FE, N − FN)
- (φ, λ) = the geodesic direct from the center with azimuth α₁ and distance ρ

## Symbols and units

`easting` and `northing` in any length unit, with the same center, falsings, and ellipsoid as the forward direction. Out come `lat` and `lon` in degrees.

## Domain

Any easting and northing. A distance past half the way around the Earth comes back from the other side.

## Approximations

None beyond the geodesic direct problem, exact to about 15 nanometers.

## Worked example

- sourcePublisher: International Association of Oil & Gas Producers (IOGP)
- sourceTitle: Coordinate Conversions and Transformations including Formulas (Guidance Note 7-2)
- sourceEdition: IOGP Publication 373-7-2, revised September 2019
- sourceLocator: Section 3.4.1, the Yap Islands example reversed
- independent: yes
- inputs: E = 42,665.90 m, N = 65,509.82 m
- outputs: 9°35′47.493″ N, 138°11′34.908″ E
- tolerance: 1e-7°
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-24

## Differential tests

- `tools/vectors/gen_azimuthal_geodesicproj.py`: GeodesicProj's coordinates for 300 random centers, points, and ellipsoids in `core/crates/gp-geodesy/tests/data/projections_azimuthal.json`, with convergence and scales from differences of those coordinates over GeographicLib ground distances, and vectors after the published examples
- `core/crates/gp-geodesy/tests/projections.rs`: `projections_match_proj` runs all 300 forward and back
- `tools/diff/runner.mjs`: `azimuthal-equidistant-forward`, 10,000 random points around twelve random centers against GeographicLib's `GeodesicProj -z`, within 1 µm; the inverse is checked by returning each fixture point from GeodesicProj's coordinates

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `projections_round_trip`: forward then inverse returns the point within 1 nm, beyond what the doubles themselves hold, over a cap of 80° around the center
- `core/crates/gp-geodesy/tests/projections.rs` `projection_invariants`: the grid distance from the center equals the geodesic distance to within 10 nm and the grid bearing from it the geodesic azimuth to 1e-9°
