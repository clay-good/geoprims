<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# ECEF to geodetic (`geodesy.frame.ecef-to-geodetic`)

## Method

Vermeille's (2011) closed form, as implemented in GeographicLib's Geocentric class: no iteration, and accurate at every height from the Earth's center outward. Longitude is atan2(Y, X). On the polar axis (X = Y = 0) the longitude is undefined: the tool returns 0 and warns LONGITUDE_UNDEFINED. At the Earth's center the latitude is undefined too, and the tool refuses the point.

## Equations

- p = (X² + Y²)/a², q = (1 − e²) Z²/a², r = (p + q − e⁴)/6.
- Solve the quartic in closed form for k, then h = (k + e² − 1)/k × √(d² + Z²) and φ = 2 atan2(Z, d + √(d² + Z²)), with d = k √(X² + Y²)/(k + e²).
- λ = atan2(Y, X).

## Symbols and units

X, Y, Z in meters; a semi-major axis (m), e² the first eccentricity squared. φ and λ in degrees, h in meters.

## Domain

Any point but the Earth's center (DEGENERATE_GEOMETRY). WGS 84 by default; any cataloged or custom ellipsoid.

## Approximations

None: a closed form in double precision. The result is within a few units in the last place of the coordinates.

## Worked example

- sourcePublisher: IOGP (International Association of Oil & Gas Producers)
- sourceTitle: Geomatics Guidance Note 7, part 2: Coordinate Conversions and Transformations including Formulas (IOGP Publication 373-7-2)
- sourceEdition: September 2019 revision
- sourceLocator: section 4.1.1 (method 9602), reverse example: X 3,771,793.968 m, Y 140,253.342 m, Z 5,124,304.349 m on WGS 84 gives 53°48'33.820"N, 2°07'46.380"E, h 73.0 m
- independent: yes
- inputs: X 3,771,793.968 m, Y 140,253.342 m, Z 5,124,304.349 m
- outputs: 53°48'33.820"N, 2°07'46.380"E, 73.0 m
- tolerance: 3e-7° (the note prints 0.001") and 0.05 m (it prints 0.1 m)
- verifiedBy: golden vector v030, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-geodesy/tests/frame_parity.rs`: `ecef_matches_cartconvert` converts 1,000 ECEF points back with GeographicLib 2.7's CartConvert -r, from the poles to beyond geostationary orbit; the worst differences are 2e-14° and 1.1e-8 m
- `tools/vectors/gen_frame_diff.py`: regenerates that fixture
- `core/vectors/geodesy.frame.ecef-to-geodetic.jsonl`: 30 vectors, including the polar axis, the center, and points near it

## Invariants

- `core/crates/gp-geodesy/tests/frames.rs` `frame_tool_invariants`: ECEF and back returns the input on three ellipsoids
- `core/crates/gp-geodesy/tests/frames.rs` `ecef_round_trips_at_every_height`: 140,000 round trips on seven ellipsoids close within 6 nm near the surface and 1e-15 relative at any height
