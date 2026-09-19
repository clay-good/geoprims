<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Destination from start, azimuth, and distance (Karney direct) (`navigation.geodesic.direct`)

## Method

Karney's direct solution through geographiclib-rs: from the start point and azimuth, the auxiliary-sphere arc for the given distance is found by inverting the I1 series (a series reversion, no iteration), and the end point and azimuth follow in closed form.

## Equations

- Reduced latitude: tan β = (1 − f) tan φ; auxiliary sphere with σ the arc length and ω the longitude.
- Distance: s12 = b [I1(σ2) − I1(σ1)], I1(σ) = A1 (σ + Σ C1l sin 2lσ), series in ε = (√(1+k²) − 1)/(√(1+k²) + 1), k = e′ cos α0.
- Longitude: λ12 = ω12 − f sin α0 [I3(σ2) − I3(σ1)], solved for α1 by Newton's method.
- Reduced length m12, scales M12 and M21 from I2; area S12 = c² (α2 − α1) + e² a² cos α0 sin α0 [I4(σ2) − I4(σ1)].

## Symbols and units

φ latitude, λ longitude, α azimuth (degrees clockwise from north), a semi-major axis, b semi-minor axis, f flattening, e′ second eccentricity; s12 distance (m), m12 reduced length (m), M12 and M21 geodesic scales (dimensionless), S12 area between the geodesic and the equator (m²).

## Domain

Any start point, azimuth, and distance, including distances past the antipode and negative distances (walking backward), on the same ellipsoids as the inverse (flattening up to 0.02).

## Approximations

None beyond double precision: about 15 nm on WGS 84 for any distance.

## Worked example

- sourcePublisher: C. F. F. Karney, GeographicLib
- sourceTitle: GeodTest-short.dat (geodesic test data computed with high-precision arithmetic)
- sourceEdition: 2010 test set, 10,000 geodesics
- sourceLocator: line 1
- independent: yes
- inputs: lat1 36.530042355041, lon1 0, azimuth 176.125875162171°, distance 9,398,502.0434687 m
- outputs: 48.164270779098° S, 5.762344694677° E, α2 175.334308316285°
- tolerance: position 1e-10° (about 11 µm); azimuth 1e-8°
- verifiedBy: automated differential test (below)
- verifiedOn: 2026-09-18

## Differential tests

- `core/crates/gp-navigation/tests/navigation.rs`: geodtest_sample_matches_karney: 1,000 GeodTest lines, end position within 15 nm (observed 6.8 nm)

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `geodesic_invariants`: symmetric distances, the direct problem inverting the inverse, and the triangle inequality, across poles and the antimeridian
