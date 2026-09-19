<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Local east-north-up of a target (`geodesy.frame.to-local`)

## Method

Convert the origin and the target to ECEF, subtract, and rotate the difference into the origin's east-north-up frame (EPSG method 9837, the same as GeographicLib's LocalCartesian). Azimuth, elevation, and range follow from east, north, and up. Straight above or below the origin the azimuth is undefined: the tool returns 0 and warns AZIMUTH_UNDEFINED. The rotation matrix is shown on request.

## Equations

- E = −ΔX sin λ0 + ΔY cos λ0.
- N = −ΔX sin φ0 cos λ0 − ΔY sin φ0 sin λ0 + ΔZ cos φ0.
- U = ΔX cos φ0 cos λ0 + ΔY cos φ0 sin λ0 + ΔZ sin φ0.
- Azimuth = atan2(E, N), elevation = atan2(U, √(E² + N²)), range = √(E² + N² + U²).

## Symbols and units

φ0, λ0, h0 the origin (degrees, degrees, meters); ΔX, ΔY, ΔZ the target's ECEF offset from the origin (m). E, N, U and range in meters; azimuth (clockwise from north) and elevation in degrees.

## Domain

Origin and target anywhere, with heights −10 km to 100,000 km. The frame is a flat tangent plane, so far targets sit well below the horizon (up is negative).

## Approximations

None: an exact rotation in double precision.

## Worked example

- sourcePublisher: IOGP (International Association of Oil & Gas Producers)
- sourceTitle: Geomatics Guidance Note 7, part 2: Coordinate Conversions and Transformations including Formulas (IOGP Publication 373-7-2)
- sourceEdition: September 2019 revision
- sourceLocator: section 4.1.3 (method 9837), example: origin 55°N, 5°E, 200 m; point 53°48'33.82"N, 2°07'46.38"E, 73.0 m gives U −189,013.869 m, V −128,642.040 m, W −4,220.171 m (east, north, up)
- independent: yes
- inputs: lat0 55, lon0 5, h0 200 m; lat 53.809394444°, lon 2.129550000°, height 73.0 m
- outputs: east −189,013.869 m, north −128,642.040 m, up −4,220.171 m
- tolerance: 5e-4 m (the note prints millimeters)
- verifiedBy: golden vector v021, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-geodesy/tests/frame_parity.rs`: `local_frames_match_cartconvert` checks 500 targets from 1 km to 1,000 km away at random origins against GeographicLib 2.7's CartConvert -l; the worst difference is 2.7e-9 m
- `tools/vectors/gen_frame_diff.py`: regenerates that fixture
- `core/vectors/geodesy.frame.to-local.jsonl`: 21 vectors from CartConvert and the guidance note

## Invariants

- `core/crates/gp-geodesy/tests/frames.rs` `frame_tool_invariants`: the origin is at zero in its own frame, range is the length of (east, north, up), and from-local returns the target
- `core/crates/gp-geodesy/tests/frames.rs` `local_frames`: the rotation is orthonormal, straight up gives elevation 90° with AZIMUTH_UNDEFINED, and 10,000 ENU round trips close within 1e-8 m
