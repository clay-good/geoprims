<!-- Derivation note (trust/correctness-program layer A). -->
# Slowest shutter speed for sharp mapping photos (`drone.photogrammetry.max-shutter`)

## Method

While the shutter is open the ground moves under the camera. The image smears by the distance flown during the exposure, measured in pixels of ground sample distance. Holding that smear to a chosen share of a pixel gives the longest usable exposure. The operation shares one calculation path with `drone.photogrammetry.motion-blur`.

## Equations

- Blur in pixels = v × t / GSD
- Longest exposure t = blur limit × GSD / v

## Symbols and units

v groundspeed; GSD ground sample distance; t exposure in seconds; the blur limit in pixels, 0.5 when left out.

## Domain

Positive GSD and groundspeed. At zero groundspeed there is no limit and the operation says so with NO_SOLUTION.

## Approximations

Forward motion only. Vibration, gimbal motion, and rolling-shutter skew are not counted.

## Worked example

- sourcePublisher: MDPI, Drones
- sourceTitle: Enhancing Direct Georeferencing Using Real-Time Kinematic UAVs and Structure from Motion-Based Photogrammetry for Large-Scale Infrastructure
- sourceEdition: volume 8, issue 12, article 736 (2024)
- sourceLocator: section 2.1.1, equation (1) and the worked example (groundspeed 10 m/s, GSD 2.0 cm); applying the 0.5 px limit gives a maximum exposure of 0.001 s, or 1/1,000 s
- independent: yes
- inputs: groundspeed 10 m/s, GSD 2.0 cm, blur limit 0.5 px
- outputs: maximum exposure 0.001 s (1/1,000 s)
- tolerance: exact
- verifiedBy: the `focused_photogrammetry_operations_match_their_general_tools` integration test, which requires the exposure to equal the stable `drone.photogrammetry.motion-blur` tool's limit for the same speed and GSD
- verifiedOn: 2026-10-17

## Differential tests

- `tools/vectors/gen_drone.py` `shutter_vectors`: 18 cases over speeds in m/s, km/h, and mph, GSDs from 0.5 to 10 cm, and limits from 0.25 to 1.5 px.
- `core/vectors/drone.photogrammetry.max-shutter.jsonl`: those vectors, the page's example, and two refusal cases.
- The integration suite requires the same limit as `drone.photogrammetry.motion-blur`.

## Invariants

- `core/crates/gp-drone/tests/drone.rs` `focused_photogrammetry_operations_match_their_general_tools`: the maximum exposure equals the stable `drone.photogrammetry.motion-blur` tool's for the same groundspeed, GSD, and blur limit.
- The exposure is proportional to the GSD and the blur limit, and inversely to the groundspeed.
