<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# UPS forward (`geodesy.ups.forward`)

## Method

UTM stops at 84° north and 80° south, because its zones converge on the poles and become useless there. Universal Polar Stereographic covers the two caps that are left. It is one polar stereographic projection per hemisphere, secant to the ellipsoid, with a scale factor of exactly 0.994 at the pole and a false easting and northing of 2,000,000 m — so the pole sits at (2000000, 2000000) and every coordinate in the cap is positive.

Grid north in UPS is the prime meridian, not the meridian through the point, so the convergence is simply the longitude: a point at 30° E has its grid north 30° away from true north. In the southern cap the grid is mirrored and the convergence is the negated longitude.

The projection is the standard ellipsoidal polar stereographic: the conformal latitude is formed from the isometric latitude, and the radius from the pole is k₀ · 2a / √((1+e)^(1+e) (1−e)^(1−e)) · tan(π/4 − χ/2). The easting and northing follow from that radius and the longitude.

## Equations

- t = tan(π/4 − φ/2) · ((1 + e sin φ)/(1 − e sin φ))^(e/2).
- ρ = 2 a k₀ t / √((1+e)^(1+e) (1−e)^(1−e)), with k₀ = 0.994.
- North: E = 2,000,000 + ρ sin λ, N = 2,000,000 − ρ cos λ.
- South: E = 2,000,000 + ρ sin λ, N = 2,000,000 + ρ cos λ.
- Convergence γ = λ in the north, −λ in the south. Scale k = k₀ ρ / (a m(φ)).

## Symbols and units

`lat` and `lon` in degrees, with an optional `ellipsoid` or explicit `a` and `inverse_flattening`. Out come the `zone` (always 0, which is what marks a coordinate as UPS rather than UTM), the `hemisphere`, `easting` and `northing` in metres, the `convergence`, the `scale`, and a `formatted` string.

## Domain

84° north and beyond, or 80° south and beyond — the region UTM does not cover. Both poles are inside the domain: at the pole the easting and northing are exactly 2,000,000 and the longitude no longer names a direction, which is the point of using a polar projection there.

## Approximations

None beyond double precision. The projection has a closed form and no series truncation; the only error is rounding.

## Worked example

- sourcePublisher: PROJ contributors; Charles Karney (GeographicLib)
- sourceTitle: PROJ, through pyproj, as EPSG:32661 and EPSG:32761 (WGS 84 / UPS North and South); GeographicLib's `GeoConvert`
- sourceEdition: PROJ 9.3.0 / pyproj 3.6.1; GeographicLib 2.7
- sourceLocator: the UPS definition — polar stereographic, k₀ = 0.994, false easting and northing 2,000,000 m
- independent: yes
- inputs: 29 points, walking the longitude circle at 84° north and at 80° south, climbing to both poles, and including the poles themselves
- outputs: the easting, northing, hemisphere and convergence in each case
- tolerance: 1e-4 m on the coordinates, 1e-9° on the convergence
- verifiedBy: golden vectors v001 to v029, run by the core on every build
- verifiedOn: 2026-09-23

Two independent implementations agree with the core and with each other at the anchor point: pyproj puts 85° N, 0° at a northing of 1444542.6086173223 and GeographicLib's `GeoConvert` prints 1444542.6086. EPSG:32661 and 32761 *are* UPS, so the reference is the projection itself rather than a transcription of its formula.

One case is deliberately left unpinned. At the antimeridian the convergence is ±180°, which is the same angle written two ways, and an implementation may report either; pinning it would be pinning a spelling. The other twenty-eight cases pin it exactly.

## Differential tests

- `tools/vectors/gen_ups.py`: 23 of the 29 vectors, from PROJ through pyproj
- `core/crates/gp-geodesy/tests/`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/geodesy.ups.forward.jsonl`: 29 vectors over both caps and both poles

## Invariants

- `core/crates/gp-geodesy/tests/geodesy.rs` `ups_invariants`: the pole lands exactly on (2,000,000, 2,000,000) in both hemispheres, which is the false origin stated as a fact rather than a constant; the forward and inverse are exact inverses away from the pole, to under a micrometre of ground; distance from the false origin depends only on latitude, so walking the longitude circle at 84° traces a circle of constant radius; the convergence is the longitude in the north and its negation in the south; a point further from the pole has a larger radius, so the projection does not fold; and the two hemispheres mirror — 85° N and 85° S at the same longitude have the same easting and northings equidistant from the origin on opposite sides
