<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Albers Equal Area inverse (`geodesy.projection.albers-inverse`)

## Method

Albers Equal Area is a conic, like Lambert's, but it keeps area instead of shape: any region has the same area on the grid as on the ellipsoid. It is the projection of the USGS and Census Bureau maps of the conterminous United States and of many land-cover and statistics grids. Parallels are arcs around the cone's apex and meridians are straight lines through it, and the scale along the parallel is the reciprocal of the scale along the meridian.

The method follows IOGP Guidance Note 7-2 (EPSG method 9822). Areas enter through q(φ), which is proportional to the area of the ellipsoid between the equator and the parallel φ. The cone constant n and the constant C come from q and m at the two standard parallels. A point's radius is a√(C − n q(φ))/n and its angle is n times its longitude from the central meridian. The guidance note recovers latitude from q with a three-term series. This tool instead solves q(φ) = q by Newton's method, started from the authalic latitude, so the result does not carry the series' truncation.

This tool is the inverse: an easting and northing back to latitude and longitude, with the same parameters as the forward direction.

## Equations

- ρ′ = ±√((E − FE)² + (ρ₀ − (N − FN))²), taking the sign of n; θ′ = atan2(±(E − FE), ±(ρ₀ − (N − FN)))
- q′ = (C − ρ′² n²/a²)/n; λ = θ′/n + λ₀
- φ solves q(φ) = q′ by Newton's method with dq/dφ = 2(1 − e²) cos φ/(1 − e² sin² φ)², started from asin(q′/q_p)

## Symbols and units

`easting` and `northing` in any length unit, with the same parameters as the forward direction. Out come `lat` and `lon` in degrees.

## Domain

An easting and northing inside the ring the poles map to. Beyond it q passes its polar value, there is no latitude, and the point is refused.

## Approximations

None in the method. Guidance Note 7-2's formulas lose digits in two places, and the tolerances say so. Standard parallels a fraction of a degree apart make n a ratio of two near-cancelling differences, which costs about a micrometer; in the worst of the 300 random cases a 50-digit evaluation puts this tool 0.4 µm off and PROJ 0.9 µm. Toward the poles the meridian scale falls to zero, so forward and back return the point within 0.1 µm to 80° of latitude and within 2 µm at 89.5°.

## Worked example

- sourcePublisher: Snyder, J. P., U.S. Geological Survey
- sourceTitle: Map Projections: A Working Manual, Professional Paper 1395
- sourceEdition: 1987
- sourceLocator: Chapter 14 numerical example, pp. 292–293 (Clarke 1866, standard parallels 29.5° and 45.5°, origin 23° N, 96° W)
- independent: yes
- inputs: x = 1,885,472.7 m, y = 1,535,925.0 m
- outputs: 35° N, 75° W
- tolerance: 0.1 m (the printed rounding); 1e-6° on the way back
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-24

The example is the publisher's own, computed independently of this tool; the tool reproduces it to the printed digits.

## Differential tests

- `tools/vectors/gen_projections_proj.py`: the grid from PROJ's aea on five ellipsoids; 300 random parameter sets and points in `core/crates/gp-geodesy/tests/data/projections_proj.json`, and vectors after the published examples
- `core/crates/gp-geodesy/tests/projections.rs`: `projections_match_proj` runs all 300 forward and back, holding the grid to 0.01 mm, the convergence to 1e-7°, and the scales to 1e-8 of themselves
- `tools/diff/runner.mjs`: `albers-forward` covers the forward direction; the inverse is checked by returning each of the 300 reference grid points to its latitude and longitude, within 0.01 mm of ground

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `projections_round_trip`: forward then inverse returns the point within 0.1 µm up to 80° of latitude (the stated tolerance for this method)
- `core/crates/gp-geodesy/tests/projections.rs` `projection_invariants`: on the central meridian the convergence is zero and the easting is the false easting; mirroring a point across the central meridian mirrors the easting; the two scales multiply to 1; and the scale along the parallel is 1 on both standard parallels
