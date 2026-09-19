<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Position from a local offset (`geodesy.frame.from-local`)

## Method

Take an offset from an origin in east-north-up, north-east-down, or azimuth-elevation-range, turn it into east-north-up, rotate it into ECEF with the transpose of the origin's rotation (EPSG method 9837, reverse), add the origin's ECEF position, and convert back to geodetic with Vermeille's closed form.

## Equations

- NED: E = east, N = north, U = −down. AER: E = r cos el sin az, N = r cos el cos az, U = r sin el.
- X = X0 − E sin λ0 − N sin φ0 cos λ0 + U cos φ0 cos λ0.
- Y = Y0 + E cos λ0 − N sin φ0 sin λ0 + U cos φ0 sin λ0.
- Z = Z0 + N cos φ0 + U sin φ0; then (X, Y, Z) to (φ, λ, h).

## Symbols and units

φ0, λ0, h0 the origin (degrees, degrees, meters); E, N, U and range r in meters; azimuth az (clockwise from north) and elevation el in degrees. Output latitude and longitude in degrees, height in meters (ellipsoidal).

## Domain

Any origin with height −10 km to 100,000 km, and offsets in one frame at a time (a frame's values must all be given). WGS 84 by default; any cataloged or custom ellipsoid.

## Approximations

None: an exact rotation and a closed-form inverse, in double precision.

## Worked example

- sourcePublisher: IOGP (International Association of Oil & Gas Producers)
- sourceTitle: Geomatics Guidance Note 7, part 2: Coordinate Conversions and Transformations including Formulas (IOGP Publication 373-7-2)
- sourceEdition: September 2019 revision
- sourceLocator: section 4.1.3 (method 9837), reverse example: origin 55°N, 5°E, 200 m; U −189,013.869 m, V −128,642.040 m, W −4,220.171 m gives 53°48'33.820"N, 2°07'46.380"E, 73.0 m
- independent: yes
- inputs: lat0 55, lon0 5, h0 200 m; east −189,013.869 m, north −128,642.040 m, up −4,220.171 m
- outputs: 53°48'33.820"N, 2°07'46.380"E, 73.0 m
- tolerance: 3e-7° (the note prints 0.001") and 0.05 m
- verifiedBy: golden vector v023, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-geodesy/tests/frame_parity.rs`: `local_frames_match_cartconvert` takes CartConvert's east-north-up for 500 targets back to geodetic and matches GeographicLib 2.7 within 1e-11° and 1e-8 m
- `tools/vectors/gen_frame_diff.py`: regenerates that fixture
- `core/vectors/geodesy.frame.from-local.jsonl`: 23 vectors from CartConvert -l -r in all three frames, and the guidance note

## Invariants

- `core/crates/gp-geodesy/tests/frames.rs` `frame_tool_invariants`: to-local then from-local returns the target on three ellipsoids
- `core/crates/gp-geodesy/tests/frames.rs` `local_frames`: AER to a position and back reproduces azimuth, elevation, and range
