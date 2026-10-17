<!-- Derivation note (trust/correctness-program layer A). -->
# Flight line spacing for a side overlap (`drone.photogrammetry.line-spacing`)

## Method

Neighboring flight lines overlap by a chosen share of the footprint across the line. The distance between lines is the rest of that footprint. The operation shares one calculation path with `drone.photogrammetry.trigger`.

## Equations

- Footprint across = s_w × h / f (s_h in portrait)
- Line spacing W = footprint across × (1 − side overlap)

## Symbols and units

h height above ground; s_w and s_h sensor width and height; f physical focal length. Sensor sizes and focal length in millimeters, heights and ground distances in meters or feet. Side overlap in percent, 60 when left out.

## Domain

Positive height, sensor size, and focal length; overlap from 0 to 99%.

## Approximations

A nadir pinhole camera over flat ground. Lens distortion, tilt, terrain relief, and crab are not modeled.

## Worked example

- sourcePublisher: Penn State College of Earth and Mineral Sciences (Q. Abdullah)
- sourceTitle: GEOG 892: Geospatial Applications of Unmanned Aerial Systems, "Designing a Flight Route"
- sourceEdition: online course text, CC BY-NC-SA 4.0, retrieved 2026-10-17
- sourceLocator: Flight Lines Computations (120 × 70 mm array, f 100 mm, 1 ft GSD at 10,000 ft, 30% side lap), line spacing 8,400 ft
- independent: yes
- inputs: height 3,048 m (10,000 ft), sensor 120 × 70 mm, f 100 mm, side overlap 30%, landscape
- outputs: across-track footprint 3,657.6 m (12,000 ft), line spacing 2,560.32 m (8,400 ft)
- tolerance: 0.01 m
- verifiedBy: the `focused_photogrammetry_operations_match_their_general_tools` integration test, which requires this spacing to equal the stable `drone.photogrammetry.trigger` tool's line spacing
- verifiedOn: 2026-10-17

## Differential tests

- `tools/vectors/gen_drone.py` `spacing_vectors`: 19 cases over eleven sensors, heights in meters and feet, overlaps from 50 to 90% and the default, landscape and portrait.
- `core/vectors/drone.photogrammetry.line-spacing.jsonl`: those vectors and two refusal cases.
- The integration suite requires the same spacing and footprint as `drone.photogrammetry.trigger`.

## Invariants

- `core/crates/gp-drone/tests/drone.rs` `focused_photogrammetry_operations_match_their_general_tools`: the across-track footprint and line spacing equal the stable `drone.photogrammetry.trigger` tool's for the same camera and side overlap.
- Spacing scales linearly with height.
- More side overlap means closer lines.
