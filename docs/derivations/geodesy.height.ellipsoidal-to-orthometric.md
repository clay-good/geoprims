<!-- Derivation note (trust/correctness-program layer A). -->
# Ellipsoidal height to orthometric height (`geodesy.height.ellipsoidal-to-orthometric`)

## Method

A GNSS height is measured from the WGS 84 ellipsoid; a map or sea-level height is measured from the geoid. The two differ by the geoid height N at the point. The operation interpolates N from the EGM96 15-minute grid and applies it in one direction, using the same grid reader and interpolation as `geodesy.height.convert`.

## Equations

- H = h − N
- N from the egm96-15 grid by GeographicLib's 12-point cubic fit (or bilinear interpolation when chosen)

## Symbols and units

h ellipsoidal height, H orthometric height, N geoid height (positive where the geoid is above the ellipsoid). Heights keep the unit they are entered in; N is reported in meters.

## Domain

Latitude from −90° to 90°, any longitude, and the ellipsoidal height h. The EGM96 grid is an on-demand asset; when it is missing the operation says so with ASSET_UNAVAILABLE.

## Approximations

EGM96 approximates global mean sea level to about half a meter to a meter and is not a national vertical datum; ORTHOMETRIC_AS_ELLIPSOIDAL says so with every result.

## Worked example

- sourcePublisher: Charles F. F. Karney
- sourceTitle: GeographicLib, GeoidEval utility
- sourceEdition: GeographicLib 2.7, with the egm96-15 grid (2009-08-29)
- sourceLocator: `GeoidEval -d assets/data/egm96-15/2009-08-29 -n egm96-15` at the point, then H = h − N
- independent: yes
- inputs: 46.452° N, 6.583° E, h = 448.2 m
- outputs: the geoid height and converted height of golden vector v001
- tolerance: 0.05 mm, GeoidEval's printed precision
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-14

## Differential tests

- `tools/vectors/gen_height_direction.py`: 20 points including both poles, heights from −400 m to 9,000 m in meters and feet, two by bilinear interpolation, each N from the GeoidEval program rather than the core's interpolator.
- `core/vectors/geodesy.height.ellipsoidal-to-orthometric.jsonl`: those vectors and two refusal cases.
- `core/crates/gp-geodesy/tests/geodesy.rs` `directional_height_conversions_match_the_general_tool` requires the general tool's answer and the round trip.

## Invariants

- Converting one way and back returns the height.
- h − H = N at every point.
- The height keeps the unit it was entered in.
