<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Hotine Oblique Mercator forward (`geodesy.projection.hotine-forward`)

## Method

A Hotine Oblique Mercator is a Mercator wrapped around the ellipsoid at a slant. Its central line, the initial line, runs through a chosen center at a chosen azimuth instead of along the equator or a meridian. Like every Mercator it is conformal, and it is true to scale (times its scale factor) along that line. That makes it the projection for regions lying along a diagonal band: the Alaska panhandle's state plane zone, the rectified skew orthomorphic grids of Malaysia and Borneo, and corridor projections.

The method follows IOGP Guidance Note 7-2. The ellipsoid is first mapped conformally onto an aposphere (constants A, B, and H), where the oblique Mercator is exact. The resulting (u, v) coordinates are then rotated by the rectified grid angle γc. The two variants differ only in where the origin of u sits. Variant A (EPSG 9812) counts u from the natural origin, where the initial line crosses the aposphere's equator, and adds the false easting and northing there. Variant B (EPSG 9815) counts u from the projection center and gives the easting and northing of the center itself. Variant A is the same code that computes State Plane's Alaska zone 1, which now runs through this shared implementation.

## Equations

- B = √(1 + e² cos⁴ φc/(1 − e²)); A = a B kc √(1 − e²)/(1 − e² sin² φc); t₀ = t(φc); D = B√(1 − e²)/(cos φc √(1 − e² sin² φc)), at least 1
- F = D + √(D² − 1)·sign(φc); H = F t₀^B; G = (F − 1/F)/2; γ₀ = asin(sin αc / D); λ₀ = λc − asin(G tan γ₀)/B
- Q = H/t^B; S = (Q − 1/Q)/2; T = (Q + 1/Q)/2; V = sin(B Δλ); U = (−V cos γ₀ + S sin γ₀)/T
- v = A ln((1 − U)/(1 + U))/(2B); u = A atan2(S cos γ₀ + V sin γ₀, cos(B Δλ))/B, less |uc|·sign(φc) in variant B, with uc = (A/B) atan(√(D² − 1)/cos αc)·sign(φc)
- E = v cos γc + u sin γc + FE; N = u cos γc − v sin γc + FN
- Convergence and scales from Richardson-extrapolated differences of the map over about 100 m

## Symbols and units

`lat`, `lon` in degrees; `variant` B (default) or A; `latitude_of_center` and `longitude_of_center` (φc, λc), `azimuth` of the initial line (αc), `rectified_grid_angle` (γc, the azimuth if not given), `scale_factor` on the initial line (kc, 1 if not given), `false_easting` and `false_northing` in any length unit (at the center for variant B, at the natural origin for A), and the ellipsoid. Out come `easting` and `northing` in meters, `convergence` in degrees, and the two scales, equal because the projection is conformal.

## Domain

A band around the initial line. The two poles of the oblique projection, 90° from the line, are at infinity and are refused, and so is a projection center at a geographic pole, where the line has no direction.

## Approximations

None in the easting and northing. The convergence and scales are finite differences, good to about 1e-9, below the digits shown.

## Worked example

- sourcePublisher: International Association of Oil & Gas Producers (IOGP)
- sourceTitle: Coordinate Conversions and Transformations including Formulas (Guidance Note 7-2)
- sourceEdition: IOGP Publication 373-7-2, revised September 2019
- sourceLocator: Section 3.2.4, the Timbalai 1948 / RSO Borneo (m) example, variant B (Everest 1830 (1967 definition), φc 4° N, λc 115° E, αc 53°18′56.9537″, γc 53°07′48.3685″, kc 0.99984, Ec 590,476.87 m, Nc 442,857.65 m)
- independent: yes
- inputs: 5°23′14.1129″ N, 115°48′19.8196″ E
- outputs: E = 679,245.73 m, N = 596,562.78 m
- tolerance: 0.01 m (the printed rounding)
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-24

## Differential tests

- `tools/vectors/gen_projections_proj.py`: 300 random grids of both variants and points within 15° of their centers, on five ellipsoids, from PROJ's omerc (variant A with +no_uoff), in `core/crates/gp-geodesy/tests/data/projections_proj.json`, and vectors after the published example
- `core/crates/gp-geodesy/tests/projections.rs`: `projections_match_proj` runs all 300 forward and back
- `tools/diff/runner.mjs`: `hotine-forward`, 10,000 random points on twelve random grids of both variants against PROJ's `proj +proj=omerc`, within 1 µm. It found that points across the antimeridian from a grid centered near it took the wrong branch; the longitude difference is now wrapped

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `projections_round_trip`: forward then inverse returns the point within 1 nm, beyond what the doubles themselves hold, over 30° around the center
- `core/crates/gp-geodesy/tests/projections.rs` `projection_invariants`: variant B puts the center at the given easting and northing; variant A with its falsings shifted by the center's variant-A coordinates gives the same grid; and the two scales agree, as a conformal map's must
