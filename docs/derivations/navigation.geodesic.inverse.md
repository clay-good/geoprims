<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Geodesic distance and azimuths (Karney inverse) (`navigation.geodesic.inverse`)

## Method

Karney's algorithm (J. Geodesy 87, 43-55, 2013) through geographiclib-rs: the geodesic is mapped to a great circle on the auxiliary sphere, the longitude difference is solved by Newton's method on the auxiliary-sphere azimuth with a robust starting guess (including the astroid solution for nearly antipodal points), and distance, reduced length, geodesic scales, and area follow from series in the third flattening.

## Equations

- Reduced latitude: tan β = (1 − f) tan φ; auxiliary sphere with σ the arc length and ω the longitude.
- Distance: s12 = b [I1(σ2) − I1(σ1)], I1(σ) = A1 (σ + Σ C1l sin 2lσ), series in ε = (√(1+k²) − 1)/(√(1+k²) + 1), k = e′ cos α0.
- Longitude: λ12 = ω12 − f sin α0 [I3(σ2) − I3(σ1)], solved for α1 by Newton's method.
- Reduced length m12, scales M12 and M21 from I2; area S12 = c² (α2 − α1) + e² a² cos α0 sin α0 [I4(σ2) − I4(σ1)].

## Symbols and units

φ latitude, λ longitude, α azimuth (degrees clockwise from north), a semi-major axis, b semi-minor axis, f flattening, e′ second eccentricity; s12 distance (m), m12 reduced length (m), M12 and M21 geodesic scales (dimensionless), S12 area between the geodesic and the equator (m²).

## Domain

Any two points, including coincident, polar, and antipodal points, on WGS 84, a named ellipsoid, a sphere, or a custom oblate ellipsoid with flattening up to 0.02 (flatter bodies are refused until the exact method is built).

## Approximations

None beyond double precision: the series are truncated at sixth order in n, about 15 nm on WGS 84. For nearly antipodal points the azimuths are ill-conditioned in the endpoints (a 1e-12° change in an endpoint can move them by 1e-5°); the distance is not.

## Worked example

- sourcePublisher: C. F. F. Karney, GeographicLib
- sourceTitle: GeodTest-short.dat (geodesic test data computed with high-precision arithmetic)
- sourceEdition: 2010 test set, 10,000 geodesics
- sourceLocator: line 1: 36.530042355041° N, 0° to 48.164270779098° S, 5.762344694677° E
- independent: yes
- inputs: lat1 36.530042355041, lon1 0, lat2 -48.16427077909777, lon2 5.762344694676511
- outputs: s12 9,398,502.0434687 m, α1 176.125875162171°, α2 175.334308316285°, m12 6,333,544.7732452 m
- tolerance: distance 20 nm; azimuths 1e-8°; reduced length 1e-7 m; area 0.5 m²
- verifiedBy: automated differential test (below)
- verifiedOn: 2026-09-18

## Differential tests

- `core/crates/gp-navigation/tests/navigation.rs`: geodtest_sample_matches_karney: 1,000 GeodTest lines, distance within 15 nm (observed 7.5 nm)
- `core/crates/gp-navigation/tests/data/GeodTest-sample.dat`: the committed sample, every 10th line of GeodTest-short.dat

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `geodesic_invariants`: symmetric distances, the direct problem inverting the inverse, and the triangle inequality, across poles and the antimeridian
