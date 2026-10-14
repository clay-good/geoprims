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

- sourcePublisher: geoprims
- sourceTitle: add-drone-suite specification
- sourceEdition: 2026
- sourceLocator: drone/photogrammetry, requirement "Overlap, trigger, and line spacing"; checked by hand arithmetic, not taken from a published table
- independent: no
- inputs: h 90 m, sensor 13.2 × 8.8 mm, f 8.8 mm, front overlap 80%
- outputs: footprint 90 m along track, spacing 18 m
- tolerance: 1e-9 m
- verifiedBy: golden vector v001 and the `focused_photogrammetry_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_drone.py` `spacing_vectors`: 19 cases over eleven sensors, heights in meters and feet, overlaps from 50 to 90% and the default, landscape and portrait.
- `core/vectors/drone.photogrammetry.trigger-distance.jsonl`: those vectors and two refusal cases.
- The integration suite requires the same spacing and footprint as `drone.photogrammetry.trigger`.

## Invariants

- Spacing scales linearly with height.
- More front overlap means a shorter spacing, reaching the whole footprint at 0%.
- Portrait swaps the sensor side used.
