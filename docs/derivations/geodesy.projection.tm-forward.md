<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Transverse Mercator forward (`geodesy.projection.tm-forward`)

## Method

The transverse Mercator wraps a cylinder around the ellipsoid, touching it along a chosen central meridian, and unrolls it. It is conformal, and true to scale (times its scale factor) along that meridian. It is the projection behind UTM, most state plane zones, the British National Grid, and the Gauss-Krüger zones of much of the world.

This tool is the same code UTM uses: Krüger's series in the third flattening, carried to sixth order as Karney (2011) gives it. That is good to 5 nanometers within 3,900 km of the central meridian, and the tool warns beyond that distance, where the error grows to meters. The central meridian, latitude of origin, scale factor, and falsings are yours to set. The northing counts from the latitude of origin by subtracting that latitude's own northing.

## Equations

- n = f/(2 − f); A = a/(1 + n)·(1 + n²/4 + n⁴/64 + n⁶/256), the rectifying radius
- τ = tan φ; τ′ = τ√(1 + σ²) − σ√(1 + τ²), σ = sinh(e atanh(eτ/√(1 + τ²))): the conformal latitude
- ξ′ = atan2(τ′, cos Δλ); η′ = asinh(sin Δλ/√(τ′² + cos² Δλ))
- ξ = ξ′ + Σ αⱼ sin 2jξ′ cosh 2jη′; η = η′ + Σ αⱼ cos 2jξ′ sinh 2jη′, j = 1…6, with Krüger's αⱼ in n
- x = k₀Aη; y = k₀Aξ; E = FE + x; N = FN + y − y(φ₀)
- Convergence and scale from the same series' derivatives (Karney 2011, eqs. 13 and 14)

## Symbols and units

`lat`, `lon` in degrees; `longitude_of_origin` (the central meridian, required), `latitude_of_origin` (0 if not given), `scale_factor` (1 if not given), `false_easting` and `false_northing` in any length unit, and the ellipsoid. Out come `easting` and `northing` in meters, `convergence` in degrees, and the two scales, equal because the projection is conformal.

## Domain

Any point; beyond 3,900 km from the central meridian (scaled by the ellipsoid's size) the result carries ACCURACY_DEGRADED.

## Approximations

The series is truncated at sixth order in n: 5 nanometers of error within 3,900 km of the central meridian, growing to meters at 70° of longitude and 15 m near 80°. The exact method (Lee 1976, as GeographicLib's TransverseMercatorExact) is a separate tool.

## Worked example

- sourcePublisher: International Association of Oil & Gas Producers (IOGP)
- sourceTitle: Coordinate Conversions and Transformations including Formulas (Guidance Note 7-2)
- sourceEdition: IOGP Publication 373-7-2, revised September 2019
- sourceLocator: Section 3.2.5, the OSGB 1936 / British National Grid example (Airy 1830, central meridian 2° W, latitude of origin 49° N, k₀ 0.9996012717, FE 400,000 m, FN −100,000 m)
- independent: yes
- inputs: 50°30′ N, 0°30′ E
- outputs: E = 577,274.99 m, N = 69,740.50 m
- tolerance: 0.01 m
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-24

The guidance note works this example with the shorter USGS series, which is some millimeters off 170 km from the central meridian. GeographicLib and PROJ both give E = 577,274.9838 m and N = 69,740.4923 m, as this tool does, which is inside the example's printed tolerance.

## Differential tests

- `tools/vectors/gen_tm_geographiclib.py`: 300 random grids and points within 35° of their central meridians, on five ellipsoids, from GeographicLib's TransverseMercatorProj -s (the same series in its author's code, with its own convergence and scale), in `core/crates/gp-geodesy/tests/data/projections_tm.json`, and vectors after the published example
- `core/crates/gp-geodesy/tests/projections.rs`: `projections_match_proj` runs all 300 forward and back
- `tools/diff/runner.mjs`: `tm-forward`, 10,000 random points on twelve random grids against TransverseMercatorProj -s, within 1 µm

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `projections_round_trip`: forward then inverse returns the point within 1 nm, beyond what the doubles themselves hold, within 30° of the central meridian
- `core/crates/gp-geodesy/tests/geodesy.rs` `utm_invariants`: the same series under UTM: mirror symmetry about the central meridian and the equator, and forward then inverse returning the point, in every zone
