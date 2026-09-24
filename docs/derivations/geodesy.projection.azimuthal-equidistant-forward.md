<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Azimuthal Equidistant forward (`geodesy.projection.azimuthal-equidistant-forward`)

## Method

An azimuthal equidistant map is centered on one place and keeps two things true from it: the distance to every other point, and the direction to it. The easting and northing are the geodesic distance from the center, split by the geodesic's starting azimuth: x = s sin α₁, y = s cos α₁. Range rings around an airport or a transmitter are circles on it, and a straight line from the center is the shortest path.

On the ellipsoid the distance and azimuth come from the exact geodesic inverse (Karney 2013), the same one the geodesic distance tool uses, so the two always agree. This is how GeographicLib's AzimuthalEquidistant class is defined, and PROJ's aeqd uses the same geodesic for an ellipsoid.

## Equations

- (s, α₁, α₂, m₁₂) = the geodesic inverse from the center (φ₀, λ₀) to the point (φ, λ)
- E = FE + s sin α₁; N = FN + s cos α₁
- Scale along the radius R = 1; across it T = s/m₁₂ (1 at the center)
- The radius reaches the point heading α₂, so the meridian is at −α₂ from it: h = √(cos² α₂ + T² sin² α₂), k = √(sin² α₂ + T² cos² α₂)
- Grid bearing of the meridian's image = α₁ + atan2(−T sin α₂, cos α₂); convergence is its negative

## Symbols and units

`lat`, `lon`, `latitude_of_origin` and `longitude_of_origin` (the center) in degrees; `false_easting` and `false_northing` in any length unit; the ellipsoid. Out come `easting` and `northing` in meters, `convergence` in degrees, and `scale_meridian` and `scale_parallel`.

## Domain

Any point. The point opposite the center is a whole circle on the map, and there the scale across the radius is infinite. Ellipsoids too flattened for the geodesic series (|f| > 0.02) are refused.

## Approximations

None beyond the geodesic inverse, which is exact to about 15 nanometers.

## Worked example

- sourcePublisher: International Association of Oil & Gas Producers (IOGP)
- sourceTitle: Coordinate Conversions and Transformations including Formulas (Guidance Note 7-2)
- sourceEdition: IOGP Publication 373-7-2, revised September 2019
- sourceLocator: Section 3.4.1, the Modified Azimuthal Equidistant example for Yap Islands (Clarke 1866). The modified method approximates the geodesic; 7 km from the center the two agree to well under a millimeter
- independent: yes
- inputs: 9°35′47.493″ N, 138°11′34.908″ E, center 9°32′48.15″ N, 138°10′07.48″ E, FE 40,000 m, FN 60,000 m
- outputs: E = 42,665.90 m, N = 65,509.82 m
- tolerance: 0.01 m (the printed rounding)
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-24

## Differential tests

- `tools/vectors/gen_azimuthal_geodesicproj.py`: GeodesicProj's coordinates for 300 random centers, points, and ellipsoids in `core/crates/gp-geodesy/tests/data/projections_azimuthal.json`, with convergence and scales from differences of those coordinates over GeographicLib ground distances, and vectors after the published examples
- `core/crates/gp-geodesy/tests/projections.rs`: `projections_match_proj` runs all 300 forward and back
- `tools/diff/runner.mjs`: `azimuthal-equidistant-forward`, 10,000 random points around twelve random centers against GeographicLib's `GeodesicProj -z`, within 1 µm

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `projections_round_trip`: forward then inverse returns the point within 1 nm, beyond what the doubles themselves hold, over a cap of 80° around the center
- `core/crates/gp-geodesy/tests/projections.rs` `projection_invariants`: the grid distance from the center equals the geodesic distance to within 10 nm and the grid bearing from it the geodesic azimuth to 1e-9°
