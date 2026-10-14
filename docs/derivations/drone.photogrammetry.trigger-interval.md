<!-- Derivation note (trust/correctness-program layer A). -->
# Camera trigger interval (`drone.photogrammetry.trigger-interval`)

## Method

At a steady groundspeed, the time between photos is the photo spacing divided by the speed. A camera that cannot shoot that fast sets a top speed instead: the spacing divided by its shortest interval. The operation shares one calculation path with `drone.photogrammetry.trigger`.

## Equations

- Photo spacing B = footprint along × (1 − front overlap)
- Interval t = B / v
- Highest groundspeed for a camera interval t_min: v_max = B / t_min

## Symbols and units

h height above ground; s_w and s_h sensor width and height; f physical focal length. Sensor sizes and focal length in millimeters, heights and ground distances in meters or feet. v groundspeed; intervals in seconds. Front overlap in percent, 75 when left out.

## Domain

Positive height, sensor size, focal length, groundspeed, and camera interval. TRIGGER_TOO_FAST warns when the interval needed is shorter than the camera's.

## Approximations

A nadir pinhole camera over flat ground. Lens distortion, tilt, terrain relief, and crab are not modeled. A time trigger keeps the overlap only at the groundspeed entered.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: add-drone-suite specification
- sourceEdition: 2026
- sourceLocator: drone/photogrammetry, scenario "Camera too slow"; checked by hand arithmetic, not taken from a published table
- independent: no
- inputs: h 90 m, sensor 13.2 × 8.8 mm, f 8.8 mm, 12 m/s, front overlap 80%, camera 2 s
- outputs: spacing 18 m, interval 1.5 s, TRIGGER_TOO_FAST, top speed 9 m/s
- tolerance: 1e-9
- verifiedBy: golden vector v001 and the `focused_photogrammetry_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_drone.py` `spacing_vectors`: 19 cases over eleven sensors, heights, overlaps, speeds from 3 to 22 m/s, and camera intervals.
- `core/vectors/drone.photogrammetry.trigger-interval.jsonl`: those vectors and four refusal cases.
- The integration suite requires the same interval, spacing, top speed, and warning as `drone.photogrammetry.trigger`.

## Invariants

- Interval × groundspeed = photo spacing.
- Doubling the groundspeed halves the interval.
- At the reported top speed the interval equals the camera's minimum.
