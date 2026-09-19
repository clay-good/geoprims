<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Geodetic to ECEF (`geodesy.frame.geodetic-to-ecef`)

## Method

The standard closed form (EPSG method 9602, forward): the prime-vertical radius of curvature at the latitude, then the point's Earth-centered, Earth-fixed coordinates. WGS 84 by default; any cataloged ellipsoid, or a custom semi-major axis and inverse flattening.

## Equations

- e² = f(2 − f), N = a / √(1 − e² sin²φ).
- X = (N + h) cos φ cos λ, Y = (N + h) cos φ sin λ, Z = (N(1 − e²) + h) sin φ.

## Symbols and units

φ latitude and λ longitude (degrees in, radians inside), h ellipsoidal height (m), a semi-major axis (m), f flattening, N prime-vertical radius (m). X, Y, Z in meters.

## Domain

Latitude −90° to 90°, any longitude, ellipsoidal height −10 km to 100,000 km (outside that, OUT_OF_DOMAIN). The height is ellipsoidal, not above sea level: convert an orthometric height with the geoid tool first.

## Approximations

None. An exact formula in double precision.

## Worked example

- sourcePublisher: IOGP (International Association of Oil & Gas Producers)
- sourceTitle: Geomatics Guidance Note 7, part 2: Coordinate Conversions and Transformations including Formulas (IOGP Publication 373-7-2)
- sourceEdition: September 2019 revision
- sourceLocator: section 4.1.1 (method 9602), example: 53°48'33.820"N, 2°07'46.380"E, h 73.0 m on WGS 84 gives X 3,771,793.968 m, Y 140,253.342 m, Z 5,124,304.349 m
- independent: yes
- inputs: lat 53.809394444°, lon 2.129550000°, height 73.0 m
- outputs: X 3,771,793.968 m, Y 140,253.342 m, Z 5,124,304.349 m
- tolerance: 5e-4 m (the note prints millimeters)
- verifiedBy: golden vector v025, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-geodesy/tests/frame_parity.rs`: `ecef_matches_cartconvert` checks 1,000 random points (heights from below the geoid to beyond geostationary orbit, one in ten at a pole) against GeographicLib 2.7's CartConvert; the worst difference is 1.5e-8 m
- `tools/vectors/gen_frame_diff.py`: regenerates that fixture
- `core/vectors/geodesy.frame.geodetic-to-ecef.jsonl`: 25 vectors from CartConvert and the guidance note

## Invariants

- `core/crates/gp-geodesy/tests/frames.rs` `frame_tool_invariants`: ECEF and back returns the input on three ellipsoids, and raising a point by h moves it exactly h
- `core/crates/gp-geodesy/tests/frames.rs` `ecef_round_trips_at_every_height`: 140,000 round trips on seven ellipsoids close within 6 nm near the surface
