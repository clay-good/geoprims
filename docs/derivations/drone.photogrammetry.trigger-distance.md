<!-- Derivation note (trust/correctness-program layer A). -->
# Photo spacing for a front overlap (`drone.photogrammetry.trigger-distance`)

## Method

Consecutive photos overlap by a chosen share of the footprint along the flight line. The distance between photo centers is the rest of that footprint. The operation shares one calculation path with `drone.photogrammetry.trigger`.

## Equations

- Footprint along = s_h × h / f (s_w in portrait)
- Photo spacing B = footprint along × (1 − front overlap)

## Symbols and units

h height above ground; s_w and s_h sensor width and height; f physical focal length. Sensor sizes and focal length in millimeters, heights and ground distances in meters or feet. Front overlap in percent, 75 when left out.

## Domain

Positive height, sensor size, and focal length; overlap from 0 to 99%.

## Approximations

A nadir pinhole camera over flat ground. Lens distortion, tilt, terrain relief, and crab are not modeled.

## Worked example

- sourcePublisher: Penn State College of Earth and Mineral Sciences (Q. Abdullah)
- sourceTitle: GEOG 892: Geospatial Applications of Unmanned Aerial Systems, "Designing a Flight Route"
- sourceEdition: online course text, CC BY-NC-SA 4.0, retrieved 2026-10-17
- sourceLocator: Aircraft Speed and Image Collection (120 × 70 mm array, f 100 mm, 1 ft GSD at 10,000 ft, 60% end lap), air base 2,800 ft
- independent: yes
- inputs: height 3,048 m (10,000 ft), sensor 120 × 70 mm, f 100 mm, front overlap 60%, landscape
- outputs: along-track footprint 2,133.6 m (7,000 ft), trigger distance 853.44 m (2,800 ft)
- tolerance: 0.01 m
- verifiedBy: the `focused_photogrammetry_operations_match_their_general_tools` integration test, which requires this spacing to equal the stable `drone.photogrammetry.trigger` tool's air base
- verifiedOn: 2026-10-17

## Differential tests

- `tools/vectors/gen_drone.py` `spacing_vectors`: 19 cases over eleven sensors, heights in meters and feet, overlaps from 50 to 90% and the default, landscape and portrait.
- `core/vectors/drone.photogrammetry.trigger-distance.jsonl`: those vectors and two refusal cases.
- The integration suite requires the same spacing and footprint as `drone.photogrammetry.trigger`.

## Invariants

- `core/crates/gp-drone/tests/drone.rs` `focused_photogrammetry_operations_match_their_general_tools`: the along-track footprint and trigger distance equal the stable `drone.photogrammetry.trigger` tool's for the same camera and front overlap.
- Spacing scales linearly with height.
- More front overlap means a shorter spacing, reaching the whole footprint at 0%.
