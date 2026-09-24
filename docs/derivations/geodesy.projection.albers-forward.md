<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Albers Equal Area forward (`geodesy.projection.albers-forward`)

## Method

Albers Equal Area is a conic, like Lambert's, but it keeps area instead of shape: any region has the same area on the grid as on the ellipsoid. It is the projection of the USGS and Census Bureau maps of the conterminous United States and of many land-cover and statistics grids. Parallels are arcs around the cone's apex and meridians are straight lines through it, and the scale along the parallel is the reciprocal of the scale along the meridian.

The method follows IOGP Guidance Note 7-2 (EPSG method 9822). Areas enter through q(φ), which is proportional to the area of the ellipsoid between the equator and the parallel φ. The cone constant n and the constant C come from q and m at the two standard parallels. A point's radius is a√(C − n q(φ))/n and its angle is n times its longitude from the central meridian. The guidance note recovers latitude from q with a three-term series. This tool instead solves q(φ) = q by Newton's method, started from the authalic latitude, so the result does not carry the series' truncation.

## Equations

- q(φ) = (1 − e²)[sin φ/(1 − e² sin² φ) − ln((1 − e sin φ)/(1 + e sin φ))/(2e)]; m(φ) = cos φ/√(1 − e² sin² φ)
- n = (m₁² − m₂²)/(q₂ − q₁), or sin φ₁ when the parallels are equal; C = m₁² + n q₁
- ρ = a√(C − n q)/n; ρ₀ = the same at the latitude of origin; θ = n Δλ
- E = FE + ρ sin θ; N = FN + ρ₀ − ρ cos θ
- Convergence = θ; scale along the parallel k = ρ n/(a m); along the meridian h = 1/k

## Symbols and units

`lat`, `lon` in degrees; `standard_parallel_1` and `_2`, `latitude_of_origin` (0 if not given), `longitude_of_origin`, `false_easting` and `false_northing` in any length unit, and the ellipsoid. Out come `easting` and `northing` in meters, `convergence` in degrees, and `scale_meridian` and `scale_parallel`, whose product is 1.

## Domain

Any point. Standard parallels mirrored about the equator make a cylinder and are refused, and so is a standard parallel at a pole.

## Approximations

None in the method. Guidance Note 7-2's formulas lose digits in two places, and the tolerances say so. Standard parallels a fraction of a degree apart make n a ratio of two near-cancelling differences, which costs about a micrometer; in the worst of the 300 random cases a 50-digit evaluation puts this tool 0.4 µm off and PROJ 0.9 µm. Toward the poles the meridian scale falls to zero, so forward and back return the point within 0.1 µm to 80° of latitude and within 2 µm at 89.5°.

## Worked example

- sourcePublisher: Snyder, J. P., U.S. Geological Survey
- sourceTitle: Map Projections: A Working Manual, Professional Paper 1395
- sourceEdition: 1987
- sourceLocator: Chapter 14 numerical example, pp. 292–293 (Clarke 1866, standard parallels 29.5° and 45.5°, origin 23° N, 96° W)
- independent: yes
- inputs: 35° N, 75° W
- outputs: x = 1,885,472.7 m, y = 1,535,925.0 m
- tolerance: 0.1 m (the printed rounding); 1e-6° on the way back
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-24

The example is the publisher's own, computed independently of this tool; the tool reproduces it to the printed digits.

## Differential tests

- `tools/vectors/gen_projections_proj.py`: the grid from PROJ's aea on five ellipsoids; 300 random parameter sets and points in `core/crates/gp-geodesy/tests/data/projections_proj.json`, and vectors after the published examples
- `core/crates/gp-geodesy/tests/projections.rs`: `projections_match_proj` runs all 300 forward and back, holding the grid to 0.01 mm, the convergence to 1e-7°, and the scales to 1e-8 of themselves
- `tools/diff/runner.mjs`: `albers-forward`, 10,000 random points on twelve random grids against PROJ's `proj +proj=aea`, within 1 µm

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `projections_round_trip`: forward then inverse returns the point within 0.1 µm up to 80° of latitude (the stated tolerance for this method)
- `core/crates/gp-geodesy/tests/projections.rs` `projection_invariants`: on the central meridian the convergence is zero and the easting is the false easting; mirroring a point across the central meridian mirrors the easting; the two scales multiply to 1; and the scale along the parallel is 1 on both standard parallels
