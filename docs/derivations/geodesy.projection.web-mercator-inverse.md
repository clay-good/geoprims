<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Web Mercator inverse (`geodesy.projection.web-mercator-inverse`)

## Method

Web Mercator is the grid of web map tiles, EPSG:3857. EPSG calls the method Popular Visualisation Pseudo Mercator (method 1024), and the name is apt: it takes the Mercator formulas for a sphere, uses the WGS 84 semi-major axis as the sphere's radius, and feeds them WGS 84 latitudes as if they were on that sphere. The result is quick to compute and makes square tiles, but it is not conformal on the ellipsoid, so north-south and east-west scales differ slightly.

The easting is the longitude, in radians, times the radius. The northing is the radius times the isometric latitude of the sphere, written here as asinh(tan φ), which is the same function as ln tan(π/4 + φ/2) but loses less to rounding near the limit. The map is cut off at the latitude where it becomes square, atan(sinh π) = 85.05112878°, where the northing equals the half circumference.

This tool is the inverse: an easting and northing back to latitude and longitude, with the same parameters as the forward direction.

## Equations

- λ = E / a (radians)
- φ = atan(sinh(N / a))

## Symbols and units

`easting` and `northing` in meters (a bare number is taken as meters and says so). Out come `lat` and `lon` in degrees on WGS 84.

## Domain

Easting and northing within ±20,037,508.34 m, the half circumference, with a millimeter of slack. Beyond that the point is off the map and is refused.

## Approximations

None: the formulas are exact, and the scales are exact for this pseudo-projection on WGS 84. They are not the sphere's 1/cos φ that PROJ's factors report, since a meter on the ground is measured on the ellipsoid.

## Worked example

- sourcePublisher: International Association of Oil & Gas Producers (IOGP)
- sourceTitle: Coordinate Conversions and Transformations including Formulas (Guidance Note 7-2)
- sourceEdition: IOGP Publication 373-7-2, revised September 2019
- sourceLocator: Section 3.4.5, the WGS 84 / Pseudo-Mercator example
- independent: yes
- inputs: E = −11,169,055.58 m, N = 2,800,000.00 m
- outputs: 24°22′54.433″ N, 100°20′00.000″ W
- tolerance: 0.01 m (the printed rounding)
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-24

The example is the publisher's own, computed independently of this tool; the tool reproduces it to the printed digits.

## Differential tests

- `tools/vectors/gen_projections_proj.py`: the grid from PROJ's webmerc; 300 random parameter sets and points in `core/crates/gp-geodesy/tests/data/projections_proj.json`, and vectors after the published examples
- `core/crates/gp-geodesy/tests/projections.rs`: `projections_match_proj` runs all 300 forward and back, holding the grid to 0.01 mm, the convergence to 1e-7°, and the scales to 1e-8 of themselves
- `tools/diff/runner.mjs`: `web-mercator-forward` covers the forward direction; the inverse is checked by returning each of the 300 reference grid points to its latitude and longitude, within 0.01 mm of ground

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `projections_round_trip`: forward then inverse returns the point within 1 nm, beyond what the doubles themselves hold, from 84° S to 84° N and a degree clear of the map's edge
- `core/crates/gp-geodesy/tests/projections.rs` `projection_invariants`: the two scales at the equator are 1 along the parallel and 1/(1 − e²) along the meridian
