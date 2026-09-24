<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Hotine Oblique Mercator inverse (`geodesy.projection.hotine-inverse`)

## Method

A Hotine Oblique Mercator is a Mercator wrapped around the ellipsoid at a slant. Its central line, the initial line, runs through a chosen center at a chosen azimuth instead of along the equator or a meridian. Like every Mercator it is conformal, and it is true to scale (times its scale factor) along that line. That makes it the projection for regions lying along a diagonal band: the Alaska panhandle's state plane zone, the rectified skew orthomorphic grids of Malaysia and Borneo, and corridor projections.

The method follows IOGP Guidance Note 7-2. The ellipsoid is first mapped conformally onto an aposphere (constants A, B, and H), where the oblique Mercator is exact. The resulting (u, v) coordinates are then rotated by the rectified grid angle γc. The two variants differ only in where the origin of u sits. Variant A (EPSG 9812) counts u from the natural origin, where the initial line crosses the aposphere's equator, and adds the false easting and northing there. Variant B (EPSG 9815) counts u from the projection center and gives the easting and northing of the center itself. Variant A is the same code that computes State Plane's Alaska zone 1, which now runs through this shared implementation.

This tool is the inverse, from an easting and northing back to latitude and longitude with the same parameters.

## Equations

- v′ = (E − FE) cos γc − (N − FN) sin γc; u′ = (N − FN) cos γc + (E − FE) sin γc, plus |uc|·sign(φc) in variant B
- Q′ = e^(−B v′/A); S′ = (Q′ − 1/Q′)/2; T′ = (Q′ + 1/Q′)/2; V′ = sin(B u′/A); U′ = (V′ cos γ₀ + S′ sin γ₀)/T′
- t′ = (H/√((1 + U′)/(1 − U′)))^(1/B); φ from t′ by fixed-point iteration to 1e-14 radians (not the guidance note's series)
- λ = λ₀ − atan2(S′ cos γ₀ − V′ sin γ₀, cos(B u′/A))/B

## Symbols and units

`easting` and `northing` in any length unit, with the same parameters as the forward direction. Out come `lat` and `lon` in degrees.

## Domain

Any easting and northing; the longitude is taken into [−180°, 180°).

## Approximations

None beyond double precision; the latitude is iterated to 1e-14 radians.

## Worked example

- sourcePublisher: International Association of Oil & Gas Producers (IOGP)
- sourceTitle: Coordinate Conversions and Transformations including Formulas (Guidance Note 7-2)
- sourceEdition: IOGP Publication 373-7-2, revised September 2019
- sourceLocator: Section 3.2.4, the Timbalai 1948 / RSO Borneo (m) example, variant B (Everest 1830 (1967 definition), φc 4° N, λc 115° E, αc 53°18′56.9537″, γc 53°07′48.3685″, kc 0.99984, Ec 590,476.87 m, Nc 442,857.65 m)
- independent: yes
- inputs: E = 679,245.73 m, N = 596,562.78 m
- outputs: 5°23′14.1129″ N, 115°48′19.8196″ E
- tolerance: 1e-7°
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-24

## Differential tests

- `tools/vectors/gen_projections_proj.py`: 300 random grids of both variants and points within 15° of their centers, on five ellipsoids, from PROJ's omerc (variant A with +no_uoff), in `core/crates/gp-geodesy/tests/data/projections_proj.json`, and vectors after the published example
- `core/crates/gp-geodesy/tests/projections.rs`: `projections_match_proj` runs all 300 forward and back
- `tools/diff/runner.mjs`: `hotine-forward`, 10,000 random points on twelve random grids of both variants against PROJ's `proj +proj=omerc`, within 1 µm. It found that points across the antimeridian from a grid centered near it took the wrong branch; the longitude difference is now wrapped

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `projections_round_trip`: forward then inverse returns the point within 1 nm, beyond what the doubles themselves hold, over 30° around the center
- `core/crates/gp-geodesy/tests/projections.rs` `projection_invariants`: variant B puts the center at the given easting and northing; variant A with its falsings shifted by the center's variant-A coordinates gives the same grid; and the two scales agree, as a conformal map's must
