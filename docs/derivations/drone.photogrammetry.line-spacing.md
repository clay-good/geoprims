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

- sourcePublisher: geoprims
- sourceTitle: add-drone-suite specification
- sourceEdition: 2026
- sourceLocator: drone/photogrammetry, requirement "Overlap, trigger, and line spacing"; checked by hand arithmetic, not taken from a published table
- independent: no
- inputs: h 90 m, sensor 13.2 × 8.8 mm, f 8.8 mm, side overlap 70%
- outputs: footprint 135 m across track, spacing 40.5 m
- tolerance: 1e-9 m
- verifiedBy: golden vector v001 and the `focused_photogrammetry_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_drone.py` `spacing_vectors`: 19 cases over eleven sensors, heights in meters and feet, overlaps from 50 to 90% and the default, landscape and portrait.
- `core/vectors/drone.photogrammetry.line-spacing.jsonl`: those vectors and two refusal cases.
- The integration suite requires the same spacing and footprint as `drone.photogrammetry.trigger`.

## Invariants

- Spacing scales linearly with height.
- More side overlap means closer lines.
- Portrait swaps the sensor side used.
