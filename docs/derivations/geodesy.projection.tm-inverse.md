<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Transverse Mercator inverse (`geodesy.projection.tm-inverse`)

## Method

The transverse Mercator wraps a cylinder around the ellipsoid, touching it along a chosen central meridian, and unrolls it. It is conformal, and true to scale (times its scale factor) along that meridian. It is the projection behind UTM, most state plane zones, the British National Grid, and the Gauss-Krüger zones of much of the world.

This tool is the same code UTM uses: Krüger's series in the third flattening, carried to sixth order as Karney (2011) gives it. That is good to 5 nanometers within 3,900 km of the central meridian, and the tool warns beyond that distance, where the error grows to meters. The central meridian, latitude of origin, scale factor, and falsings are yours to set. The northing counts from the latitude of origin by subtracting that latitude's own northing.

This tool is the inverse, from an easting and northing back to latitude and longitude with the same parameters.

## Equations

- ξ = (N − FN + y(φ₀))/(k₀A); η = (E − FE)/(k₀A)
- ξ′ = ξ − Σ βⱼ sin 2jξ cosh 2jη; η′ = η − Σ βⱼ cos 2jξ sinh 2jη, j = 1…6
- τ′ = sin ξ′/√(sinh² η′ + cos² ξ′); Δλ = atan2(sinh η′, cos ξ′)
- τ from τ′ by Newton's method (Karney 2011, eqs. 19 to 21); φ = atan τ; λ = λ₀ + Δλ

## Symbols and units

`easting` and `northing` in any length unit, with the same parameters as the forward direction. Out come `lat` and `lon` in degrees.

## Domain

Any easting and northing; beyond 3,900 km from the central meridian the result carries ACCURACY_DEGRADED.

## Approximations

The series is truncated at sixth order in n: 5 nanometers of error within 3,900 km of the central meridian, growing to meters at 70° of longitude and 15 m near 80°. The exact method (Lee 1976, as GeographicLib's TransverseMercatorExact) is a separate tool.

## Worked example

- sourcePublisher: International Association of Oil & Gas Producers (IOGP)
- sourceTitle: Coordinate Conversions and Transformations including Formulas (Guidance Note 7-2)
- sourceEdition: IOGP Publication 373-7-2, revised September 2019
- sourceLocator: Section 3.2.5, the OSGB 1936 / British National Grid example (Airy 1830, central meridian 2° W, latitude of origin 49° N, k₀ 0.9996012717, FE 400,000 m, FN −100,000 m)
- independent: yes
- inputs: E = 577,274.99 m, N = 69,740.50 m
- outputs: 50°30′ N, 0°30′ E
- tolerance: 1e-6°
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
