<!-- Derivation note (trust/correctness-program layer A). -->
# Image footprint on the ground (`drone.photogrammetry.footprint`)

## Method

A frame camera pointed straight down sees a rectangle of ground. By similar triangles, each side of that rectangle is the matching side of the sensor scaled by the height over the focal length. The operation reports both sides and the area. It uses the same camera model as `drone.photogrammetry.gsd`, without the pixel counts a footprint does not need.

## Equations

- Footprint across = s_w × h / f
- Footprint along = s_h × h / f
- Area = across × along
- From a 35 mm-equivalent focal length: f = f_eq ÷ crop factor

## Symbols and units

h height above ground; s_w and s_h sensor width and height; f physical focal length. Sensor sizes and focal length in millimeters, heights and ground distances in meters or feet.

## Domain

Positive height, sensor size, and focal length. A 35 mm-equivalent focal length needs the crop factor or is converted from the sensor size.

## Approximations

A nadir pinhole camera over flat ground. Lens distortion, tilt, terrain relief, and crab are not modeled.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: add-drone-suite specification
- sourceEdition: 2026
- sourceLocator: drone/photogrammetry, requirement "GSD and footprint"; checked by hand arithmetic, not taken from a published table
- independent: no
- inputs: h 90 m, sensor 13.2 × 8.8 mm, f 8.8 mm
- outputs: 135 m across, 90 m along, 12,150 m²
- tolerance: 1e-9 m
- verifiedBy: golden vectors and the `focused_photogrammetry_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_drone.py` `footprint_vectors`: 19 cases over eleven sensors and heights in meters and feet, some entered by 35 mm-equivalent focal length, by a separate Python evaluation.
- `core/vectors/drone.photogrammetry.footprint.jsonl`: those vectors and two refusal cases.
- The integration suite requires the same footprints as `drone.photogrammetry.gsd`.

## Invariants

- Both sides scale linearly with height.
- Across ÷ along equals the sensor's width ÷ height.
- A 35 mm-equivalent focal length gives the same footprint as its physical value.
