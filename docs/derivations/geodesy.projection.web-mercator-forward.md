<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Web Mercator forward (`geodesy.projection.web-mercator-forward`)

## Method

Web Mercator is the grid of web map tiles, EPSG:3857. EPSG calls the method Popular Visualisation Pseudo Mercator (method 1024), and the name is apt: it takes the Mercator formulas for a sphere, uses the WGS 84 semi-major axis as the sphere's radius, and feeds them WGS 84 latitudes as if they were on that sphere. The result is quick to compute and makes square tiles, but it is not conformal on the ellipsoid, so north-south and east-west scales differ slightly.

The easting is the longitude, in radians, times the radius. The northing is the radius times the isometric latitude of the sphere, written here as asinh(tan φ), which is the same function as ln tan(π/4 + φ/2) but loses less to rounding near the limit. The map is cut off at the latitude where it becomes square, atan(sinh π) = 85.05112878°, where the northing equals the half circumference.

## Equations

- E = a·Δλ, with Δλ the longitude difference in radians, taken into [−π, π)
- N = a·asinh(tan φ)
- Scale along the parallel k = a / (ν cos φ); along the meridian h = a / (ρ cos φ), with ν = a/√(1 − e² sin² φ) and ρ = a(1 − e²)/(1 − e² sin² φ)^1.5 the WGS 84 radii of curvature
- Convergence is zero: meridians are vertical lines

## Symbols and units

`lat` and `lon` in degrees on WGS 84. Out come `easting` and `northing` in meters, `convergence` in degrees, and the dimensionless `scale_meridian` (h) and `scale_parallel` (k). a = 6,378,137 m, and e² is WGS 84's.

## Domain

Latitudes within ±85.05112878°. Beyond it the point is off the map and the result is OUT_OF_DOMAIN, stating the limit.

## Approximations

None: the formulas are exact, and the scales are exact for this pseudo-projection on WGS 84. They are not the sphere's 1/cos φ that PROJ's factors report, since a meter on the ground is measured on the ellipsoid.

## Worked example

- sourcePublisher: International Association of Oil & Gas Producers (IOGP)
- sourceTitle: Coordinate Conversions and Transformations including Formulas (Guidance Note 7-2)
- sourceEdition: IOGP Publication 373-7-2, revised September 2019
- sourceLocator: Section 3.4.5, the WGS 84 / Pseudo-Mercator example
- independent: yes
- inputs: 24°22′54.433″ N, 100°20′00.000″ W
- outputs: E = −11,169,055.58 m, N = 2,800,000.00 m
- tolerance: 0.01 m (the printed rounding)
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-24

The example is the publisher's own, computed independently of this tool; the tool reproduces it to the printed digits.

## Differential tests

- `tools/vectors/gen_projections_proj.py`: the grid from PROJ's webmerc; 300 random parameter sets and points in `core/crates/gp-geodesy/tests/data/projections_proj.json`, and vectors after the published examples
- `core/crates/gp-geodesy/tests/projections.rs`: `projections_match_proj` runs all 300 forward and back, holding the grid to 0.01 mm, the convergence to 1e-7°, and the scales to 1e-8 of themselves
- `tools/diff/runner.mjs`: `web-mercator-forward`, 10,000 random points on the map against PROJ's `proj +proj=webmerc`, within 1 µm

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `projections_round_trip`: forward then inverse returns the point within 1 nm, beyond what the doubles themselves hold, from 84° S to 84° N and a degree clear of the map's edge
- `core/crates/gp-geodesy/tests/projections.rs` `projection_invariants`: the two scales at the equator are 1 along the parallel and 1/(1 − e²) along the meridian
