<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# UTM to latitude and longitude (`geodesy.utm.inverse`)

## Method

The inverse Krüger series (βj coefficients) from the grid to the conformal sphere, then τ from τ′ by Newton's method (at most five iterations to machine precision, Karney 2011 eq. 19-21), with convergence and scale from the same derivatives as the forward direction.

## Equations

- Conformal latitude: τ′ = τ √(1 + σ²) − σ √(1 + τ²), τ = tan φ, σ = sinh(e atanh(e τ/√(1 + τ²))).
- ξ′ = atan2(τ′, cos λ), η′ = asinh(sin λ / √(τ′² + cos² λ)).
- ξ = ξ′ + Σ αj sin 2jξ′ cosh 2jη′, η = η′ + Σ αj cos 2jξ′ sinh 2jη′ (j = 1…6).
- E = 500,000 + k0 A η, N = k0 A ξ (+ 10,000,000 m south), A the rectifying radius; convergence γ and scale k from the derivatives of the series.

## Symbols and units

φ latitude, λ longitude from the central meridian, e eccentricity, n third flattening, A rectifying radius (m), k0 = 0.9996, E easting and N northing (m), γ grid convergence (degrees), k point scale factor.

## Domain

80° S to 84° N (UPS covers the poles). A zone may be forced up to 3 zones from the standard one, with a NONSTANDARD_ZONE warning; farther is refused.

## Approximations

Krüger's sixth-order series is accurate to 5 nm within 3,900 km of the central meridian (Karney 2011), which covers every zone and three zones either side.

## Worked example

- sourcePublisher: C. F. F. Karney, GeographicLib
- sourceTitle: TMcoords.dat (exact transverse Mercator computed with 80-digit arithmetic)
- sourceEdition: 2009 test set, 287,000 points
- sourceLocator: eighth point of the committed sample
- independent: yes
- inputs: zone 31 N, easting and northing from TMcoords
- outputs: latitude and longitude from TMcoords
- tolerance: 1e-12° in latitude and longitude; convergence 1e-12°; scale 1e-13
- verifiedBy: automated differential test (below)
- verifiedOn: 2026-09-18

## Differential tests

- `core/crates/gp-geodesy/tests/geodesy.rs`: utm_matches_exact_transverse_mercator: 1,000 TMcoords points, forward and inverse, within 5 nm (observed 3.7 nm)
- `core/crates/gp-geodesy/tests/data/TMcoords-sample.dat`: the committed sample within 3.5° of the central meridian

## Invariants

- `core/crates/gp-geodesy/tests/geodesy.rs` `utm_invariants`: mirror symmetry about the central meridian and the equator, and forward then inverse returning the point, in every tenth zone
