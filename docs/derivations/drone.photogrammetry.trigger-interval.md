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

- sourcePublisher: Penn State College of Earth and Mineral Sciences (Q. Abdullah)
- sourceTitle: GEOG 892: Geospatial Applications of Unmanned Aerial Systems, "Designing a Flight Route"
- sourceEdition: online course text, CC BY-NC-SA 4.0, retrieved 2026-10-17
- sourceLocator: Aircraft Speed and Image Collection (120 × 70 mm array, f 100 mm, 1 ft GSD at 10,000 ft, 60% end lap, 150 knots), ~11.07 s between exposures
- independent: yes
- inputs: height 3,048 m (10,000 ft), sensor 120 × 70 mm, f 100 mm, front overlap 60%, groundspeed 150 kt, landscape
- outputs: trigger distance 853.44 m (2,800 ft), interval 11.060 s (the course prints 11.067 s, differing only by its 1.15 mph-per-knot rounding where the core uses the exact 1,852 m per nautical mile)
- tolerance: 0.01 s against the exact-conversion value
- verifiedBy: the `focused_photogrammetry_operations_match_their_general_tools` integration test, which requires this interval to equal the stable `drone.photogrammetry.trigger` tool's
- verifiedOn: 2026-10-17

## Differential tests

- `tools/vectors/gen_drone.py` `spacing_vectors`: 19 cases over eleven sensors, heights, overlaps, speeds from 3 to 22 m/s, and camera intervals.
- `core/vectors/drone.photogrammetry.trigger-interval.jsonl`: those vectors and four refusal cases.
- The integration suite requires the same interval, spacing, top speed, and warning as `drone.photogrammetry.trigger`.

## Invariants

- `core/crates/gp-drone/tests/drone.rs` `focused_photogrammetry_operations_match_their_general_tools`: the interval equals the stable `drone.photogrammetry.trigger` tool's for the same camera, overlap, and groundspeed.
- Interval × groundspeed = photo spacing.
- Doubling the groundspeed halves the interval.
