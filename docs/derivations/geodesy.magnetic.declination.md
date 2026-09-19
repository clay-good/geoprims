<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Magnetic declination (WMM2025 and IGRF-14) (`geodesy.magnetic.declination`)

## Method

Spherical-harmonic synthesis of the geomagnetic main field from Gauss coefficients, as in the WMM Technical Report: geodetic to geocentric coordinates, Schmidt semi-normalized associated Legendre functions, the field sum, a pole-safe east component, and rotation back to the geodetic frame. Secular variation uses the same sum with the coefficient rates.

## Equations

- Coefficients at time t: g(t) = g(t0) + ġ·(t − t0), h likewise (WMM); IGRF interpolates linearly between 5-year epochs and uses its secular variation after 2025.
- Potential sum: X′ = −Σ (a/r)^(n+2) (g cos mλ + h sin mλ) dPₙᵐ/dφ′, Y′ = Σ (a/r)^(n+2) m (g sin mλ − h cos mλ) Pₙᵐ / cos φ′, Z′ = −Σ (n+1)(a/r)^(n+2) (g cos mλ + h sin mλ) Pₙᵐ.
- Rotation: X = X′ cos ψ − Z′ sin ψ, Z = X′ sin ψ + Z′ cos ψ, ψ = φ′ − φ.
- Elements: H = √(X²+Y²), F = √(H²+Z²), D = atan2(Y, X), I = atan2(Z, H).
- WMM declination uncertainty: √(0.26² + (5417/H)²) degrees.

## Symbols and units

φ geodetic latitude, φ′ geocentric latitude, λ longitude, r geocentric radius (km), a = 6371.2 km reference radius, g and h Gauss coefficients (nT), X north, Y east, Z down, H horizontal, F total (nT), D declination, I inclination (degrees).

## Domain

Heights -1 km to 850 km above the WGS 84 ellipsoid. WMM2025: 2025.0 to 2030.0. IGRF-14: 1900.0 to 2030.0. H under 1 nT is refused as undefined declination.

## Approximations

Main field only: crustal anomalies of several degrees are not modeled. Linear secular variation within each model's window.

## Worked example

- sourcePublisher: NOAA National Centers for Environmental Information
- sourceTitle: WMM2025 test values (WMM2025_TestValues.txt)
- sourceEdition: WMM2025, released 2024-11-13
- sourceLocator: Row 2: 2025.0, 48 km, 80° N, 96° W
- independent: yes
- inputs: lat 80, lon -96, height 48 km, date 2025.0
- outputs: D -29.91°, I 87.77°, H 2,164.285547 nT
- tolerance: D and I 0.005° (printed to 0.01°); intensities 0.001 nT
- verifiedBy: automated differential test (below)
- verifiedOn: 2026-09-18

## Differential tests

- `core/crates/gp-geodesy/tests/magnetic.rs`: all 100 NCEI WMM2025 test values, every element and rate
- `core/vectors/geodesy.magnetic.declination.jsonl`: IGRF-14 against ppigrf 2.1 at coefficient epochs

## Invariants

- `core/crates/gp-geodesy/tests/magnetic.rs` `field_element_invariants`: H² = X² + Y², F² = H² + Z², D = atan2(Y, X), I = atan2(Z, H) for both models; the ±180° meridian agrees
