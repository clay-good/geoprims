<!-- Derivation note (trust/correctness-program layer A). -->
# Point scale factor in UTM (`geodesy.projection.scale-factor`)

## Method

Universal Transverse Mercator is a transverse Mercator projection in six-degree zones with a scale of 0.9996 on each central meridian. Away from that meridian the grid's north lines lean from true north by the convergence, and distances are stretched by the point scale factor. The operation reports the scale first, from the same projection call as `geodesy.utm.forward`.

## Equations

- Central meridian λ₀ = 6° × zone − 183°
- Convergence γ and scale k from the transverse Mercator series (Karney 2011); to first order γ ≈ (λ − λ₀)·sin φ and k ≈ 0.9996·(1 + (λ − λ₀)² cos² φ / 2), λ in radians
- True azimuth = grid azimuth + γ; grid distance = ellipsoid distance × k

## Symbols and units

φ, λ latitude and longitude in degrees; γ in degrees, positive when grid north lies east of true north; k unitless.

## Domain

UTM's range, 80° S to 84° N. A forced zone may be up to three zones from the standard one.

## Approximations

The sixth-order Krüger series, which agrees with the exact projection to well under a nanometer inside a zone.

## Worked example

- sourcePublisher: Charles F. F. Karney
- sourceTitle: GeographicLib, TransverseMercatorProj utility
- sourceEdition: GeographicLib 2.7
- sourceLocator: `TransverseMercatorProj -t -l <central meridian> -k 0.9996 -p 10`, the exact transverse Mercator by elliptic functions rather than the series
- independent: yes
- inputs: 60.3913° N, 5.3221° E in zone 32 on WGS 84
- outputs: scale 1.0001032402, convergence −3.1987166918°
- tolerance: 1e-9
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_geodesy_focused.py` `grid_figure_vectors`: 20 points from 79° S to 78° N, in standard and forced zones and on three ellipsoids, each solved by GeographicLib's exact transverse Mercator at the zone's central meridian.
- `core/vectors/geodesy.projection.scale-factor.jsonl`: those vectors and two refusal cases.
- The integration suite requires the same convergence, scale, and zone as `geodesy.utm.forward`.

## Invariants

- `core/crates/gp-geodesy/tests/geodesy_views_invariants.rs` `projection_views_invariants`: on the central meridian the scale is 0.9996 and the convergence 0; off it the scale is larger, and the same at equal distances either side; the convergence changes sign across the central meridian and the equator.
