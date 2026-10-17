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

- sourcePublisher: Alshaibani, Helvaci, Shayea, Saad, Azizan, and Yakub
- sourceTitle: Airplane Type Identification Based on Mask RCNN and Drone Images (arXiv:2108.12811)
- sourceEdition: arXiv v1, 2021
- sourceLocator: Equation 1 and figure 5 (1-inch sensor 12.75 × 8.5 mm, 10.6 mm lens, 120 m), 3.13 cm/px over the 4,608 × 3,072 px frame
- independent: yes
- inputs: height 120 m, sensor 12.75 × 8.5 mm, focal length 10.6 mm
- outputs: 144.34 m across, 96.23 m along, 13,889 m² (the published 3.13 cm/px over 4,608 × 3,072 px)
- tolerance: 0.2 m (the footprint implied by the paper's printed GSD and pixel counts)
- verifiedBy: the `focused_photogrammetry_operations_match_their_general_tools` integration test, which requires these footprints to equal the stable `drone.photogrammetry.gsd` tool's
- verifiedOn: 2026-10-17

## Differential tests

- `tools/vectors/gen_drone.py` `footprint_vectors`: 19 cases over eleven sensors and heights in meters and feet, some entered by 35 mm-equivalent focal length, by a separate Python evaluation.
- `core/vectors/drone.photogrammetry.footprint.jsonl`: those vectors and two refusal cases.
- The integration suite requires the same footprints as `drone.photogrammetry.gsd`.

## Invariants

- `core/crates/gp-drone/tests/drone.rs` `focused_photogrammetry_operations_match_their_general_tools`: the across- and along-track footprints and their product equal the stable `drone.photogrammetry.gsd` tool's for the same camera.
- Across ÷ along equals the sensor's width ÷ height.
- A 35 mm-equivalent focal length gives the same footprint as its physical value.
