<!-- Derivation note (trust/correctness-program layer A). -->
# Meridian arc length (`geodesy.ellipsoid.meridian-arc`)

## Method

The meridian arc is the distance along a line of longitude on the ellipsoid. From the equator to a latitude it is the integral of the meridional radius of curvature, which the core evaluates through the rectifying latitude. The operation reports that arc, the arc between two latitudes, and the local length of a degree, from the same functions as `geodesy.ellipsoid.radii`.

## Equations

- M(φ) = a(1 − e²) ∫₀^φ (1 − e² sin² t)^(−3/2) dt
- Arc between φ₁ and φ₂ = M(φ₂) − M(φ₁)
- Degree of latitude at φ = M(φ + ½°) − M(φ − ½°)

## Symbols and units

φ geodetic latitude in degrees; a semi-major axis; e² first eccentricity squared. Lengths in meters, positive northward.

## Domain

Latitudes from −90° to 90°, on a catalog ellipsoid or a custom one given by a and inverse flattening.

## Approximations

The series is accurate to well under a millimeter on Earth-like ellipsoids.

## Worked example

- sourcePublisher: Charles F. F. Karney
- sourceTitle: GeographicLib, GeodSolve utility
- sourceEdition: GeographicLib 2.7
- sourceLocator: `GeodSolve -i -p 9` from 0°, 0° to 40°, 0° and to 41°, 0°: a geodesic along a meridian is the meridian arc
- independent: yes
- inputs: latitudes 40° and 41° on WGS 84
- outputs: 4,429,529.030 m from the equator to 40°, and 111,044.261 m on to 41°
- tolerance: 2e-6 m
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_geodesy_focused.py` `arc_vectors`: 20 latitudes and pairs on WGS 84, GRS 80, Clarke 1866, International 1924, and Bessel 1841, each arc solved by the GeodSolve command-line program as a geodesic along the prime meridian.
- `core/vectors/geodesy.ellipsoid.meridian-arc.jsonl`: those vectors and two refusal cases.
- The integration suite requires the same arcs as `geodesy.ellipsoid.radii` and checks the WGS 84 quadrant, 10,001,965.729 m.

## Invariants

- `core/crates/gp-geodesy/tests/geodesy_views_invariants.rs` `ellipsoid_views_invariants`: M(−φ) = −M(φ), the arc grows with latitude, and the arc between two latitudes is the difference of their arcs from the equator.
