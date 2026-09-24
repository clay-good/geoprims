<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Equidistant Cylindrical inverse (`geodesy.projection.equidistant-cylindrical-inverse`)

## Method

The Equidistant Cylindrical, or plate carrée, spaces the parallels by their true distance along the meridian, so a northing is the ground distance from the equator. The easting is the longitude times the radius of one chosen parallel, the standard parallel, so east-west distances are true there and nowhere else. Meridians and parallels are straight lines at right angles, and grid north is true north everywhere.

This is EPSG's ellipsoidal method 1028, used by EPSG:4087 (WGS 84 / World Equidistant Cylindrical). The meridian distance comes from Krüger's series for the transverse Mercator, whose central meridian is true to scale: it is good to nanometers and is the same code the UTM tools use, so the math is not written twice. PROJ's eqc is the spherical form, EPSG method 1029, whose northing is the latitude times the semi-major axis. On WGS 84 the two differ by up to 21 km at the poles, so this method is checked against GeographicLib's meridian distance instead.

This tool is the inverse: an easting and northing back to latitude and longitude, with the same parameters as the forward direction.

## Equations

- φ = M⁻¹(N − FN) by Krüger's inverse series
- λ = λ₀ + (E − FE)/(ν₁ cos φ₁), taken into [−180°, 180°)

## Symbols and units

`easting` and `northing` in any length unit, with the same parameters as the forward direction. Out come `lat` and `lon` in degrees.

## Domain

A northing within the meridian quadrant of the false northing (10,001,965.73 m on WGS 84); beyond it the point is past the pole and is refused. An easting past half the parallel's circumference wraps around.

## Approximations

The meridian distance is a sixth-order series in the third flattening, good to nanometers on any Earth-like ellipsoid; otherwise exact.

## Worked example

- sourcePublisher: International Association of Oil & Gas Producers (IOGP)
- sourceTitle: Coordinate Conversions and Transformations including Formulas (Guidance Note 7-2)
- sourceEdition: IOGP Publication 373-7-2, revised September 2019
- sourceLocator: Section 3.3.3, the WGS 84 / World Equidistant Cylindrical example
- independent: yes
- inputs: E = 1,113,194.91 m, N = 6,097,230.31 m
- outputs: 55° N, 10° E
- tolerance: 0.01 m on the grid; 1e-7° on the way back
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-24

The example is the publisher's own, computed independently of this tool; the tool reproduces it to the printed digits.

## Differential tests

- `tools/vectors/gen_projections_proj.py`: the northing from GeographicLib's geodesic along the meridian and the easting from the standard parallel's radius, on five ellipsoids; 300 random parameter sets and points in `core/crates/gp-geodesy/tests/data/projections_proj.json`, and vectors after the published examples
- `core/crates/gp-geodesy/tests/projections.rs`: `projections_match_proj` runs all 300 forward and back, holding the grid to 0.01 mm, the convergence to 1e-7°, and the scales to 1e-8 of themselves

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `projections_round_trip`: forward then inverse returns the point within 1 nm, beyond what the doubles themselves hold, from 84° S to 84° N
- `core/crates/gp-geodesy/tests/projections.rs` `projection_invariants`: on the central meridian the easting is the false easting; mirroring a point across the central meridian mirrors the easting; the scale along the meridian is exactly 1; and the scale along the parallel is 1 on the standard parallel
