<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Geoid height (EGM96) (`geodesy.geoid.geoid-height`)

## Method

Interpolation in the EGM96 15′ grid distributed by GeographicLib, exactly as GeographicLib's Geoid::height: a 12-point least-squares cubic fit (with pole-adapted stencils) or bilinear interpolation, with reflection across the poles.

## Equations

- Grid value: N = offset + scale · v, with v the 16-bit sample (offset -108 m, scale 0.003 m).
- Cubic: t = C · v over the 12-point stencil, N = t₀ + x(t₁ + x(t₃ + x t₆)) + y(t₂ + x(t₄ + x t₇) + y(t₅ + x t₈ + y t₉)).
- Bilinear: N = (1−y)((1−x)v₀₀ + x v₀₁) + y((1−x)v₁₀ + x v₁₁).

## Symbols and units

N geoid height above the WGS 84 ellipsoid (m); x, y fractional grid position; C GeographicLib's fit matrix.

## Domain

Every latitude and longitude. The grid is a data asset checked by SHA-256 before use.

## Approximations

The 15′ grid departs from full EGM96 by up to 0.17 m (cubic) or 1.15 m (bilinear), as the grid header states. EGM96 approximates mean sea level to about 0.5-1 m; it is not NAVD 88.

## Worked example

- sourcePublisher: C. F. F. Karney, GeographicLib
- sourceTitle: GeoidEval with the egm96-15 grid
- sourceEdition: GeographicLib 2.7; egm96-15 grid of 2009-08-29
- sourceLocator: GeoidEval -n egm96-15 at 16.776° N, 3.009° W
- independent: yes
- inputs: lat 16.776, lon -3.009
- outputs: 28.7079 m
- tolerance: 0.00005 m (GeoidEval prints 4 decimals)
- verifiedBy: automated differential test (below)
- verifiedOn: 2026-09-18

## Differential tests

- `core/crates/gp-geodesy/tests/geoid.rs`: 2,010 points including both poles, cubic and bilinear, against GeoidEval

## Invariants

- `core/crates/gp-geodesy/tests/geoid.rs` `geoid_height_invariants`: one value per pole, the ±180° meridian agrees, values in EGM96's range, cubic and bilinear within the stated errors
