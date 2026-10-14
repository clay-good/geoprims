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

- sourcePublisher: geoprims
- sourceTitle: add-drone-suite specification
- sourceEdition: 2026
- sourceLocator: drone/photogrammetry, requirement "Motion blur"; checked by hand arithmetic, not taken from a published table
- independent: no
- inputs: 12 m/s, GSD 2.5 cm, limit 0.5 px
- outputs: 1.25 cm of travel, 1/960 s
- tolerance: 1e-12 s
- verifiedBy: golden vector v001 and the `focused_photogrammetry_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_drone.py` `shutter_vectors`: 18 cases over speeds in m/s, km/h, and mph, GSDs from 0.5 to 10 cm, and limits from 0.25 to 1.5 px.
- `core/vectors/drone.photogrammetry.max-shutter.jsonl`: those vectors, the page's example, and two refusal cases.
- The integration suite requires the same limit as `drone.photogrammetry.motion-blur`.

## Invariants

- The exposure is proportional to the GSD and the blur limit, and inversely to the groundspeed.
- At the reported exposure the blur equals the limit.
