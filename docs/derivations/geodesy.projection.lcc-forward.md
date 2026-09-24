<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Lambert Conformal Conic forward (`geodesy.projection.lcc-forward`)

## Method

A Lambert Conformal Conic wraps a cone around the ellipsoid, touching it along one parallel (the one-parallel variant, EPSG method 9801) or cutting it along two (the two-parallel variant, method 9802), and unrolls it. It is conformal: at any point the scale is the same in every direction, so small shapes and angles are kept. Parallels become arcs of circles around the cone's apex and meridians become straight lines through it.

The method follows IOGP Guidance Note 7-2. The cone constant n and the mapping radius come from the isometric-latitude function t(φ) at the standard parallels. A point's radius r is proportional to t(φ)^n, and its angle around the apex is n times its longitude from the central meridian. The two-parallel variant is true to scale on both standard parallels. The one-parallel variant is true to scale, times its scale factor, on the parallel of the origin. The latitude of origin is where northings count from (for the two-parallel variant, the false origin).

## Equations

- t(φ) = tan(π/4 − φ/2) / ((1 − e sin φ)/(1 + e sin φ))^(e/2); m(φ) = cos φ / √(1 − e² sin² φ)
- 2SP: n = (ln m₁ − ln m₂)/(ln t₁ − ln t₂); F = m₁/(n t₁ⁿ). 1SP: n = sin φ₀; F = m₀/(n t₀ⁿ), times k₀
- r = a F tⁿ; r_F = a F t_Fⁿ at the latitude of origin; θ = n Δλ
- E = FE + r sin θ; N = FN + r_F − r cos θ
- Convergence = θ; scale h = k = r n / (a m)

## Symbols and units

`lat`, `lon` in degrees; `variant` 2SP (default) or 1SP; `standard_parallel_1` and `_2` (2SP), `latitude_of_origin` (the false origin for 2SP, the natural origin for 1SP), `longitude_of_origin`, `scale_factor` (1SP only), `false_easting` and `false_northing` in any length unit, and the ellipsoid. Out come `easting` and `northing` in meters, `convergence` in degrees, and `scale_meridian` and `scale_parallel`, equal because the projection is conformal.

## Domain

Any point except the pole away from the cone's apex, which is at infinity and is refused. A parameter that belongs to the other variant is refused rather than ignored, and standard parallels mirrored about the equator, which make a cylinder, are refused with a pointer to the cylindrical methods.

## Approximations

None beyond double precision. The inverse latitude is iterated to convergence rather than stopped at a fixed number of terms.

## Worked example

- sourcePublisher: International Association of Oil & Gas Producers (IOGP)
- sourceTitle: Coordinate Conversions and Transformations including Formulas (Guidance Note 7-2)
- sourceEdition: IOGP Publication 373-7-2, revised September 2019
- sourceLocator: Section 3.2.1.1 (NAD27 / Texas South Central, two parallels) and 3.2.1.2 (JAD69 / Jamaica National Grid, one parallel)
- independent: yes
- inputs: 28°30′ N, 96°00′ W on Texas South Central; 17°55′55.80″ N, 76°56′37.26″ W on the Jamaica grid
- outputs: E = 2,963,503.91 ftUS, N = 254,759.80 ftUS; E = 255,966.58 m, N = 142,493.51 m
- tolerance: 0.01 m on the grid; 1e-7° on the way back
- verifiedBy: golden vectors v001 and v002, run by the core on every build
- verifiedOn: 2026-09-24

The example is the publisher's own, computed independently of this tool; the tool reproduces it to the printed digits.

## Differential tests

- `tools/vectors/gen_projections_proj.py`: the grid from PROJ's lcc, both variants, on five ellipsoids; 300 random parameter sets and points in `core/crates/gp-geodesy/tests/data/projections_proj.json`, and vectors after the published examples
- `core/crates/gp-geodesy/tests/projections.rs`: `projections_match_proj` runs all 300 forward and back, holding the grid to 0.01 mm, the convergence to 1e-7°, and the scales to 1e-8 of themselves
- `tools/diff/runner.mjs`: `lcc-forward`, 10,000 random points on twelve random grids of both variants against PROJ's `proj +proj=lcc`, within 1 µm

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `projections_round_trip`: forward then inverse returns the point within 1 nm, beyond what the doubles themselves hold, over the whole cone except a degree either side of the cut opposite the central meridian, in both variants
- `core/crates/gp-geodesy/tests/projections.rs` `projection_invariants`: on the central meridian the convergence is zero and the easting is the false easting; mirroring a point across the central meridian mirrors the easting and keeps the northing; the two scales are equal; and the two-parallel variant is true to scale on both standard parallels
